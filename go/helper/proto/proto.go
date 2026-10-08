// Package proto is the wire format between fmtkit and fmtkit-go-helper. The
// layout is specified in PROTOCOL.md; the Rust codec in crates/go mirrors it,
// and both sides test against the frames in testdata/.
package proto

import (
	"encoding/binary"
	"errors"
	"fmt"
	"io"
)

// Kind tags what a frame carries.
type Kind uint8

// Frame is one decoded frame with its payload still encoded.
type Frame struct {
	Kind    Kind
	ID      uint32
	Payload []byte
}

// Hello is the first frame each way.
type Hello struct {
	Proto   uint32
	Version string
}

// Steps selects what the helper does to one file.
type Steps struct {
	Spacing        bool
	Gofmt          bool
	Goimports      bool
	ResolveImports bool
	Complexity     bool
}

// Request asks the helper to process one file.
type Request struct {
	Rel    string
	Abs    string
	Source []byte
	Steps  Steps
}

// Violation is one spacing finding. Line and Column are 1-based; 0 means none.
type Violation struct {
	Rule    string
	Line    uint32
	Column  uint32
	Message string
}

// Score is one function's complexity.
type Score struct {
	Key        string
	Name       string
	Line       uint32
	Cyclomatic uint32
	Cognitive  uint32
}

// Reply answers one Request. Error is empty when the file was processed.
type Reply struct {
	Error      string
	Output     []byte
	Applied    []string
	Violations []Violation
	Complexity []Score
}

type encoder struct {
	buf []byte
}

// decoder reads a payload front to back. The first failure sticks: later reads
// return zero values, and finish reports it.
type decoder struct {
	buf []byte
	err error
}

// Version is the protocol version exchanged in the Hello handshake. Bump it on
// any change to the frame or payload layout.
const Version uint32 = 1

// MaxFrame caps the length field of a frame, so a corrupt length cannot make
// either side allocate without bound.
const MaxFrame = 256 << 20

// headerLen is the kind byte plus the id that follow the length field.
const headerLen = 5

// The frame kinds. Hello opens the stream in both directions, Process and
// Reply pair up by id, and Shutdown asks the helper to finish and exit.
const (
	KindHello    Kind = 1
	KindProcess  Kind = 2
	KindReply    Kind = 3
	KindShutdown Kind = 4
)

// ErrMalformed wraps every decoding failure.
var ErrMalformed = errors.New("malformed frame")

func malformed(format string, args ...any) error {
	return fmt.Errorf("%w: %s", ErrMalformed, fmt.Sprintf(format, args...))
}

// ReadFrame reads one frame. It returns io.EOF only when the stream ends
// cleanly between frames.
func ReadFrame(r io.Reader) (Frame, error) {
	var prefix [4]byte

	if _, err := io.ReadFull(r, prefix[:]); err != nil {
		if errors.Is(err, io.EOF) {
			return Frame{}, io.EOF
		}

		return Frame{}, malformed("truncated length: %v", err)
	}

	length := binary.LittleEndian.Uint32(prefix[:])

	if length < headerLen {
		return Frame{}, malformed("frame length %d is shorter than its header", length)
	}

	if length > MaxFrame {
		return Frame{}, malformed("frame length %d exceeds %d", length, MaxFrame)
	}

	body := make([]byte, length)

	if _, err := io.ReadFull(r, body); err != nil {
		return Frame{}, malformed("truncated frame: %v", err)
	}

	return Frame{
		Kind:    Kind(body[0]),
		ID:      binary.LittleEndian.Uint32(body[1:headerLen]),
		Payload: body[headerLen:],
	}, nil
}

// AppendFrame appends one encoded frame to dst.
func AppendFrame(dst []byte, kind Kind, id uint32, payload []byte) []byte {
	dst = binary.LittleEndian.AppendUint32(dst, uint32(headerLen+len(payload)))
	dst = append(dst, byte(kind))
	dst = binary.LittleEndian.AppendUint32(dst, id)

	return append(dst, payload...)
}

// WriteFrame writes one encoded frame to w.
func WriteFrame(w io.Writer, kind Kind, id uint32, payload []byte) error {
	_, err := w.Write(AppendFrame(nil, kind, id, payload))

	return err
}

// EncodeHello encodes a Hello payload.
func EncodeHello(hello Hello) []byte {
	var e encoder

	e.u32(hello.Proto)
	e.str(hello.Version)

	return e.buf
}

// DecodeHello decodes a Hello payload.
func DecodeHello(payload []byte) (Hello, error) {
	d := decoder{buf: payload}
	hello := Hello{Proto: d.u32(), Version: d.str()}

	return hello, d.finish("hello")
}

// EncodeRequest encodes a Process payload.
func EncodeRequest(req Request) []byte {
	var e encoder

	e.str(req.Rel)
	e.str(req.Abs)
	e.bytes(req.Source)
	e.boolean(req.Steps.Spacing)
	e.boolean(req.Steps.Gofmt)
	e.boolean(req.Steps.Goimports)
	e.boolean(req.Steps.ResolveImports)
	e.boolean(req.Steps.Complexity)

	return e.buf
}

// DecodeRequest decodes a Process payload.
func DecodeRequest(payload []byte) (Request, error) {
	d := decoder{buf: payload}

	req := Request{
		Rel:    d.str(),
		Abs:    d.str(),
		Source: d.bytes(),
	}

	req.Steps = Steps{
		Spacing:        d.boolean(),
		Gofmt:          d.boolean(),
		Goimports:      d.boolean(),
		ResolveImports: d.boolean(),
		Complexity:     d.boolean(),
	}

	return req, d.finish("process request")
}

// EncodeReply encodes a Reply payload.
func EncodeReply(reply Reply) []byte {
	var e encoder

	e.str(reply.Error)
	e.bytes(reply.Output)
	e.count(len(reply.Applied))

	for _, name := range reply.Applied {
		e.str(name)
	}

	e.count(len(reply.Violations))

	for _, v := range reply.Violations {
		e.str(v.Rule)
		e.u32(v.Line)
		e.u32(v.Column)
		e.str(v.Message)
	}

	e.count(len(reply.Complexity))

	for _, s := range reply.Complexity {
		e.str(s.Key)
		e.str(s.Name)
		e.u32(s.Line)
		e.u32(s.Cyclomatic)
		e.u32(s.Cognitive)
	}

	return e.buf
}

// DecodeReply decodes a Reply payload.
func DecodeReply(payload []byte) (Reply, error) {
	d := decoder{buf: payload}
	reply := Reply{Error: d.str(), Output: d.bytes()}

	for range d.count(4) {
		reply.Applied = append(reply.Applied, d.str())
	}

	for range d.count(16) {
		reply.Violations = append(reply.Violations, Violation{
			Rule:    d.str(),
			Line:    d.u32(),
			Column:  d.u32(),
			Message: d.str(),
		})
	}

	for range d.count(20) {
		reply.Complexity = append(reply.Complexity, Score{
			Key:        d.str(),
			Name:       d.str(),
			Line:       d.u32(),
			Cyclomatic: d.u32(),
			Cognitive:  d.u32(),
		})
	}

	return reply, d.finish("reply")
}

func (e *encoder) u32(v uint32) {
	e.buf = binary.LittleEndian.AppendUint32(e.buf, v)
}

func (e *encoder) count(n int) {
	e.u32(uint32(n))
}

func (e *encoder) bytes(b []byte) {
	e.count(len(b))
	e.buf = append(e.buf, b...)
}

func (e *encoder) str(s string) {
	e.count(len(s))
	e.buf = append(e.buf, s...)
}

func (e *encoder) boolean(b bool) {
	if b {
		e.buf = append(e.buf, 1)

		return
	}

	e.buf = append(e.buf, 0)
}

func (d *decoder) fail(format string, args ...any) {
	if d.err == nil {
		d.err = malformed(format, args...)
	}

	d.buf = nil
}

func (d *decoder) take(n uint32) []byte {
	if d.err != nil {
		return nil
	}

	if uint64(n) > uint64(len(d.buf)) {
		d.fail("need %d bytes, have %d", n, len(d.buf))

		return nil
	}

	out := d.buf[:n:n]
	d.buf = d.buf[n:]

	return out
}

func (d *decoder) u32() uint32 {
	b := d.take(4)

	if b == nil {
		return 0
	}

	return binary.LittleEndian.Uint32(b)
}

func (d *decoder) bytes() []byte {
	n := d.u32()

	return d.take(n)
}

func (d *decoder) str() string {
	return string(d.bytes())
}

func (d *decoder) boolean() bool {
	b := d.take(1)

	if b == nil {
		return false
	}

	switch b[0] {
	case 0:
		return false
	case 1:
		return true
	default:
		d.fail("bool byte %d", b[0])

		return false
	}
}

// count reads a list length and rejects one that the remaining payload cannot
// hold, given the smallest encoded size of one element.
func (d *decoder) count(minSize uint32) int {
	n := d.u32()

	if d.err != nil {
		return 0
	}

	if uint64(n)*uint64(minSize) > uint64(len(d.buf)) {
		d.fail("list of %d elements does not fit in %d bytes", n, len(d.buf))

		return 0
	}

	return int(n)
}

func (d *decoder) finish(what string) error {
	if d.err != nil {
		return fmt.Errorf("%s: %w", what, d.err)
	}

	if len(d.buf) != 0 {
		return fmt.Errorf("%s: %w", what, malformed("%d trailing bytes", len(d.buf)))
	}

	return nil
}

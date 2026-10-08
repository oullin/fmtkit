package main

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"strings"
	"sync"
	"testing"
	"time"

	"go.ollin.sh/fmtkit/go/helper/format"
	"go.ollin.sh/fmtkit/go/helper/proto"
)

// session drives serve over in-memory pipes the way fmtkit does.
type session struct {
	t    *testing.T
	in   *io.PipeWriter
	out  *io.PipeReader
	done chan error
}

// failingWriter accepts the hello and fails every write after it.
type failingWriter struct {
	mu     sync.Mutex
	writes int
}

func start(t *testing.T, workers int, handle handler) *session {
	t.Helper()

	inR, inW := io.Pipe()
	outR, outW := io.Pipe()
	s := &session{t: t, in: inW, out: outR, done: make(chan error, 1)}

	go func() {
		err := serve(inR, outW, workers, handle)

		outW.Close()

		s.done <- err
	}()

	s.send(proto.KindHello, 0, proto.EncodeHello(proto.Hello{Proto: proto.Version, Version: "test"}))

	hello := s.read()

	if hello.Kind != proto.KindHello || hello.ID != 0 {
		t.Fatalf("first frame = %#v", hello)
	}

	got, err := proto.DecodeHello(hello.Payload)

	if err != nil || got.Proto != proto.Version || got.Version != version {
		t.Fatalf("hello = %#v, %v", got, err)
	}

	return s
}

func (s *session) send(kind proto.Kind, id uint32, payload []byte) {
	s.t.Helper()

	if err := proto.WriteFrame(s.in, kind, id, payload); err != nil {
		s.t.Fatalf("write frame: %v", err)
	}
}

func (s *session) read() proto.Frame {
	s.t.Helper()

	frame, err := proto.ReadFrame(s.out)

	if err != nil {
		s.t.Fatalf("read frame: %v", err)
	}

	return frame
}

func (s *session) reply() (uint32, proto.Reply) {
	s.t.Helper()

	frame := s.read()

	if frame.Kind != proto.KindReply {
		s.t.Fatalf("frame kind = %d", frame.Kind)
	}

	reply, err := proto.DecodeReply(frame.Payload)

	if err != nil {
		s.t.Fatalf("decode reply: %v", err)
	}

	return frame.ID, reply
}

func (s *session) finish() error {
	s.t.Helper()

	// Drain whatever is left so the writer never blocks.
	go func() { _, _ = io.Copy(io.Discard, s.out) }()

	select {
	case err := <-s.done:
		return err
	case <-time.After(10 * time.Second):
		s.t.Fatal("serve did not return")

		return nil
	}
}

func request(src string) []byte {
	return proto.EncodeRequest(proto.Request{Rel: "a.go", Abs: "/tmp/a.go", Source: []byte(src), Steps: proto.Steps{Spacing: true, Gofmt: true, Complexity: true}})
}

func TestServeFormatsRequests(t *testing.T) {
	s := start(t, 2, format.Process)

	s.send(proto.KindProcess, 42, request("package a\nfunc f( ) {}\n"))

	id, reply := s.reply()

	if id != 42 || reply.Error != "" || string(reply.Output) != "package a\n\nfunc f() {}\n" {
		t.Fatalf("reply %d = %#v", id, reply)
	}

	s.send(proto.KindShutdown, 0, nil)

	if err := s.finish(); err != nil {
		t.Fatalf("serve: %v", err)
	}
}

func TestServeRepliesOutOfOrder(t *testing.T) {
	release := make(chan struct{})

	handle := func(req proto.Request) proto.Reply {
		if string(req.Source) == "slow" {
			<-release
		}

		return proto.Reply{Output: req.Source}
	}

	s := start(t, 2, handle)

	s.send(proto.KindProcess, 1, request("slow"))
	s.send(proto.KindProcess, 2, request("fast"))

	if id, _ := s.reply(); id != 2 {
		t.Fatalf("first reply id = %d, want 2", id)
	}

	close(release)

	if id, reply := s.reply(); id != 1 || string(reply.Output) != "slow" {
		t.Fatalf("second reply = %d %#v", id, reply)
	}

	s.in.Close()

	if err := s.finish(); err != nil {
		t.Fatalf("serve: %v", err)
	}
}

func TestServeAnswersEveryRequestBeforeExiting(t *testing.T) {
	s := start(t, 4, func(req proto.Request) proto.Reply {
		time.Sleep(time.Millisecond)

		return proto.Reply{Output: req.Source}
	})

	const count = 200

	go func() {
		for i := range count {
			_ = proto.WriteFrame(s.in, proto.KindProcess, uint32(i+1), request(fmt.Sprint(i)))
		}

		_ = proto.WriteFrame(s.in, proto.KindShutdown, 0, nil)
	}()

	seen := map[uint32]bool{}

	for range count {
		id, reply := s.reply()

		if string(reply.Output) != fmt.Sprint(id-1) {
			t.Fatalf("reply %d carries %q", id, reply.Output)
		}

		seen[id] = true
	}

	if len(seen) != count {
		t.Fatalf("saw %d distinct replies", len(seen))
	}

	if err := s.finish(); err != nil {
		t.Fatalf("serve: %v", err)
	}
}

func TestServeTurnsAPanicIntoAnErrorReply(t *testing.T) {
	s := start(t, 1, func(req proto.Request) proto.Reply {
		if string(req.Source) == "boom" {
			panic("kaboom")
		}

		return proto.Reply{Output: req.Source}
	})

	s.send(proto.KindProcess, 1, request("boom"))

	if id, reply := s.reply(); id != 1 || !strings.Contains(reply.Error, "kaboom") || string(reply.Output) != "boom" {
		t.Fatalf("reply = %d %#v", id, reply)
	}

	s.send(proto.KindProcess, 2, request("fine"))

	if id, reply := s.reply(); id != 2 || reply.Error != "" {
		t.Fatalf("the worker did not survive the panic: %d %#v", id, reply)
	}

	s.send(proto.KindShutdown, 0, nil)

	if err := s.finish(); err != nil {
		t.Fatalf("serve: %v", err)
	}
}

func TestServeRejectsMalformedFrames(t *testing.T) {
	cases := map[string]func(s *session){
		"bad request":   func(s *session) { s.send(proto.KindProcess, 1, []byte{1, 2, 3}) },
		"unknown kind":  func(s *session) { s.send(proto.Kind(99), 1, nil) },
		"reply to them": func(s *session) { s.send(proto.KindReply, 1, proto.EncodeReply(proto.Reply{})) },
		"short frame": func(s *session) {
			if _, err := s.in.Write([]byte{2, 0, 0, 0, 0, 0}); err != nil {
				s.t.Fatalf("write: %v", err)
			}
		},
	}

	for name, send := range cases {
		t.Run(name, func(t *testing.T) {
			s := start(t, 1, format.Process)

			send(s)

			if err := s.finish(); err == nil {
				t.Fatal("serve accepted a malformed frame")
			}
		})
	}
}

func TestServeRequiresHelloFirst(t *testing.T) {
	var out bytes.Buffer

	in := proto.AppendFrame(nil, proto.KindShutdown, 0, nil)

	if err := serve(bytes.NewReader(in), &out, 1, format.Process); err == nil {
		t.Fatal("serve accepted a stream without hello")
	}

	if err := serve(bytes.NewReader(nil), &out, 1, format.Process); err != nil {
		t.Fatalf("an empty stream is a clean exit: %v", err)
	}
}

func (w *failingWriter) Write(p []byte) (int, error) {
	w.mu.Lock()

	defer w.mu.Unlock()

	w.writes++

	if w.writes == 1 {
		return len(p), nil
	}

	return 0, errors.New("closed")
}

func TestServeReportsWriteFailuresWithoutHanging(t *testing.T) {
	var in []byte

	in = proto.AppendFrame(in, proto.KindHello, 0, proto.EncodeHello(proto.Hello{Proto: proto.Version}))

	for i := range 50 {
		in = proto.AppendFrame(in, proto.KindProcess, uint32(i+1), request("package a\n"))
	}

	err := serve(bytes.NewReader(in), &failingWriter{}, 2, format.Process)

	if err == nil || !strings.Contains(err.Error(), "write reply") {
		t.Fatalf("err = %v", err)
	}
}

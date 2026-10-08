package proto_test

import (
	"bytes"
	"errors"
	"flag"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"go.ollin.sh/fmtkit/go/helper/proto"
)

// fixture is one golden frame. The same values are spelled out in the Rust
// codec tests (crates/go/src/proto.rs), so both codecs agree byte for byte.
type fixture struct {
	name    string
	kind    proto.Kind
	id      uint32
	payload func() []byte
	check   func(t *testing.T, payload []byte)
}

var updateFixtures = flag.Bool("update", false, "rewrite the golden protocol frames in testdata/")

var helloFixture = proto.Hello{Proto: 1, Version: "2.0.0"}

var requestFixture = proto.Request{
	Rel:    "pkg/a.go",
	Abs:    "/repo/pkg/a.go",
	Source: []byte("package a\n"),
	Steps:  proto.Steps{Spacing: true, Gofmt: true, Complexity: true},
}

var replyFixture = proto.Reply{
	Output:  []byte("package a\n\nfunc f() {}\n"),
	Applied: []string{"spacing", "gofmt"},
	Violations: []proto.Violation{
		{Rule: "spacing", Line: 3, Message: "missing blank line"},
		{Rule: "spacing", Line: 10, Column: 2, Message: "ünïcode ✓"},
	},
	Complexity: []proto.Score{
		{Key: "pkg/a.go#f", Name: "f", Line: 3, Cyclomatic: 1},
		{Key: "pkg/a.go#(*T).M", Name: "(*T).M", Line: 5, Cyclomatic: 4, Cognitive: 6},
	},
}

var errorReplyFixture = proto.Reply{Error: "gofmt: 1:1: expected 'package', found 'EOF'"}

func fixtures() []fixture {
	return []fixture{
		{
			name:    "hello",
			kind:    proto.KindHello,
			payload: func() []byte { return proto.EncodeHello(helloFixture) },
			check: func(t *testing.T, payload []byte) {
				got, err := proto.DecodeHello(payload)

				if err != nil || got != helloFixture {
					t.Fatalf("hello = %#v, %v", got, err)
				}
			},
		},
		{
			name:    "process",
			kind:    proto.KindProcess,
			id:      7,
			payload: func() []byte { return proto.EncodeRequest(requestFixture) },
			check: func(t *testing.T, payload []byte) {
				got, err := proto.DecodeRequest(payload)

				if err != nil || !reflect.DeepEqual(got, requestFixture) {
					t.Fatalf("request = %#v, %v", got, err)
				}
			},
		},
		{
			name:    "reply",
			kind:    proto.KindReply,
			id:      7,
			payload: func() []byte { return proto.EncodeReply(replyFixture) },
			check: func(t *testing.T, payload []byte) {
				got, err := proto.DecodeReply(payload)

				if err != nil || !reflect.DeepEqual(got, replyFixture) {
					t.Fatalf("reply = %#v, %v", got, err)
				}
			},
		},
		{
			name:    "reply_error",
			kind:    proto.KindReply,
			id:      0xFFFF_FFFF,
			payload: func() []byte { return proto.EncodeReply(errorReplyFixture) },
			check: func(t *testing.T, payload []byte) {
				got, err := proto.DecodeReply(payload)

				if err != nil || got.Error != errorReplyFixture.Error || len(got.Output) != 0 || got.Applied != nil || got.Violations != nil || got.Complexity != nil {
					t.Fatalf("reply = %#v, %v", got, err)
				}
			},
		},
		{
			name:    "shutdown",
			kind:    proto.KindShutdown,
			payload: func() []byte { return nil },
			check: func(t *testing.T, payload []byte) {
				if len(payload) != 0 {
					t.Fatalf("payload = %v", payload)
				}
			},
		},
	}
}

func TestGoldenFrames(t *testing.T) {
	for _, fx := range fixtures() {
		t.Run(fx.name, func(t *testing.T) {
			path := filepath.Join("testdata", fx.name+".bin")
			encoded := proto.AppendFrame(nil, fx.kind, fx.id, fx.payload())

			if *updateFixtures {
				if err := os.WriteFile(path, encoded, 0o644); err != nil {
					t.Fatalf("write fixture: %v", err)
				}
			}

			want, err := os.ReadFile(path)

			if err != nil {
				t.Fatalf("read fixture: %v", err)
			}

			if !bytes.Equal(encoded, want) {
				t.Fatalf("encoded frame differs from %s\n got: %x\nwant: %x", path, encoded, want)
			}

			frame, err := proto.ReadFrame(bytes.NewReader(want))

			if err != nil {
				t.Fatalf("read frame: %v", err)
			}

			if frame.Kind != fx.kind || frame.ID != fx.id {
				t.Fatalf("frame header = (%d, %d), want (%d, %d)", frame.Kind, frame.ID, fx.kind, fx.id)
			}

			fx.check(t, frame.Payload)
		})
	}
}

func TestReadFrameStopsCleanlyBetweenFrames(t *testing.T) {
	stream := proto.AppendFrame(nil, proto.KindShutdown, 0, nil)
	stream = proto.AppendFrame(stream, proto.KindHello, 1, proto.EncodeHello(helloFixture))
	reader := bytes.NewReader(stream)

	for range 2 {
		if _, err := proto.ReadFrame(reader); err != nil {
			t.Fatalf("read frame: %v", err)
		}
	}

	if _, err := proto.ReadFrame(reader); !errors.Is(err, io.EOF) {
		t.Fatalf("err = %v, want io.EOF", err)
	}
}

func TestReadFrameRejectsBadInput(t *testing.T) {
	cases := map[string][]byte{
		"truncated length":  {1, 0},
		"short header":      {4, 0, 0, 0, 1, 0, 0, 0},
		"oversized":         {0xFF, 0xFF, 0xFF, 0xFF},
		"truncated payload": {9, 0, 0, 0, 1, 0, 0, 0, 0, 1},
	}

	for name, input := range cases {
		t.Run(name, func(t *testing.T) {
			_, err := proto.ReadFrame(bytes.NewReader(input))

			if !errors.Is(err, proto.ErrMalformed) {
				t.Fatalf("err = %v, want ErrMalformed", err)
			}
		})
	}
}

func TestDecodersRejectBadPayloads(t *testing.T) {
	request := proto.EncodeRequest(requestFixture)
	badBool := bytes.Clone(request)
	badBool[len(badBool)-1] = 2
	reply := proto.EncodeReply(replyFixture)

	hello := func(payload []byte) error {
		_, err := proto.DecodeHello(payload)

		return err
	}

	process := func(payload []byte) error {
		_, err := proto.DecodeRequest(payload)

		return err
	}

	answer := func(payload []byte) error {
		_, err := proto.DecodeReply(payload)

		return err
	}

	cases := map[string]func() error{
		"empty hello":         func() error { return hello(nil) },
		"trailing hello":      func() error { return hello(append(proto.EncodeHello(helloFixture), 0)) },
		"bad bool":            func() error { return process(badBool) },
		"truncated request":   func() error { return process(request[:len(request)-1]) },
		"truncated reply":     func() error { return answer(reply[:len(reply)-3]) },
		"huge list":           func() error { return answer([]byte{0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF}) },
		"string past the end": func() error { return hello([]byte{1, 0, 0, 0, 9, 0, 0, 0, 'x'}) },
	}

	for name, decode := range cases {
		t.Run(name, func(t *testing.T) {
			if err := decode(); !errors.Is(err, proto.ErrMalformed) {
				t.Fatalf("err = %v, want ErrMalformed", err)
			}
		})
	}
}

func FuzzDecodeReply(f *testing.F) {
	f.Add(proto.EncodeReply(replyFixture))
	f.Add(proto.EncodeReply(errorReplyFixture))

	f.Fuzz(func(t *testing.T, payload []byte) {
		reply, err := proto.DecodeReply(payload)

		if err != nil {
			return
		}

		if !bytes.Equal(proto.EncodeReply(reply), payload) {
			t.Fatalf("decode/encode does not round-trip")
		}
	})
}

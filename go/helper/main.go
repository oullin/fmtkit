// Command fmtkit-go-helper runs fmtkit's Go formatting steps for the fmtkit
// binary. It speaks the framed protocol in proto/PROTOCOL.md over stdin and
// stdout and is not meant to be run by hand.
package main

import (
	"bufio"
	"errors"
	"fmt"
	"io"
	"os"
	"runtime"
	"runtime/debug"
	"sync"

	"go.ollin.sh/fmtkit/go/helper/format"
	"go.ollin.sh/fmtkit/go/helper/proto"
)

// handler processes one request; serve recovers its panics.
type handler func(proto.Request) proto.Reply

// job is one decoded request waiting for a worker.
type job struct {
	id  uint32
	req proto.Request
}

// version is the fmtkit release this helper ships with, stamped at build time
// with -ldflags "-X main.version=<version>". Unstamped builds report "dev".
var version = "dev"

func main() {
	if len(os.Args) > 1 && (os.Args[1] == "--version" || os.Args[1] == "version") {
		fmt.Printf("fmtkit-go-helper %s (protocol %d)\n", version, proto.Version)

		return
	}

	if err := serve(os.Stdin, os.Stdout, runtime.GOMAXPROCS(0), format.Process); err != nil {
		fmt.Fprintf(os.Stderr, "fmtkit-go-helper: %v\n", err)
		os.Exit(1)
	}
}

// serve answers the handshake, then runs one reader (this goroutine), a pool
// of workers, and one writer until Shutdown or the end of input. Replies go out
// in completion order, tagged with their request id. It returns once every
// accepted request has been answered.
func serve(in io.Reader, out io.Writer, workers int, handle handler) error {
	reader := bufio.NewReaderSize(in, 1<<16)
	frame, err := proto.ReadFrame(reader)

	if errors.Is(err, io.EOF) {
		return nil
	}

	if err != nil {
		return err
	}

	if frame.Kind != proto.KindHello {
		return fmt.Errorf("expected hello, got frame kind %d", frame.Kind)
	}

	if _, err := proto.DecodeHello(frame.Payload); err != nil {
		return err
	}

	// The caller judges compatibility; the helper only states what it is.
	if err := proto.WriteFrame(out, proto.KindHello, 0, proto.EncodeHello(proto.Hello{Proto: proto.Version, Version: version})); err != nil {
		return fmt.Errorf("write hello: %w", err)
	}

	jobs := make(chan job, workers*4)
	replies := make(chan []byte, workers*4)
	writeErr := make(chan error, 1)

	go func() {
		writeErr <- writeReplies(out, replies)
	}()

	var pool sync.WaitGroup

	for range max(workers, 1) {
		pool.Go(func() {
			for j := range jobs {
				replies <- proto.AppendFrame(nil, proto.KindReply, j.id, proto.EncodeReply(run(handle, j.req)))
			}
		})
	}

	readErr := readJobs(reader, jobs)

	close(jobs)
	pool.Wait()
	close(replies)

	return errors.Join(readErr, <-writeErr)
}

// readJobs decodes Process frames into jobs until Shutdown or a clean end of
// input. Anything else is a protocol error.
func readJobs(reader io.Reader, jobs chan<- job) error {
	for {
		frame, err := proto.ReadFrame(reader)

		if errors.Is(err, io.EOF) {
			return nil
		}

		if err != nil {
			return err
		}

		switch frame.Kind {
		case proto.KindShutdown:
			return nil
		case proto.KindProcess:
			req, err := proto.DecodeRequest(frame.Payload)

			if err != nil {
				return fmt.Errorf("request %d: %w", frame.ID, err)
			}

			jobs <- job{id: frame.ID, req: req}
		default:
			return fmt.Errorf("unexpected frame kind %d", frame.Kind)
		}
	}
}

// writeReplies writes every reply frame, flushing whenever the queue drains.
// After a write error it keeps draining so the workers never block.
func writeReplies(out io.Writer, replies <-chan []byte) error {
	writer := bufio.NewWriterSize(out, 1<<16)

	var failed error

	for frame := range replies {
		if failed != nil {
			continue
		}

		if _, err := writer.Write(frame); err != nil {
			failed = fmt.Errorf("write reply: %w", err)

			continue
		}

		if len(replies) == 0 {
			if err := writer.Flush(); err != nil {
				failed = fmt.Errorf("write reply: %w", err)
			}
		}
	}

	if failed != nil {
		return failed
	}

	if err := writer.Flush(); err != nil {
		return fmt.Errorf("write reply: %w", err)
	}

	return nil
}

// run calls handle, turning a panic into an error reply for this request
// alone. The stack goes to stderr, which fmtkit only surfaces on a crash.
func run(handle handler, req proto.Request) (reply proto.Reply) {
	defer func() {
		if recovered := recover(); recovered != nil {
			fmt.Fprintf(os.Stderr, "fmtkit-go-helper: panic on %s: %v\n%s", req.Rel, recovered, debug.Stack())

			reply = proto.Reply{
				Error:  fmt.Sprintf("internal error: %v", recovered),
				Output: req.Source,
			}
		}
	}()

	return handle(req)
}

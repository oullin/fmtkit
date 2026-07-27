package console

import (
	"strings"
	"testing"
)

func TestDetectColorHonorsForceColor(t *testing.T) {
	t.Setenv("NO_COLOR", "1")
	t.Setenv("FORCE_COLOR", "1")

	if got := DetectColor(&strings.Builder{}); got != ColorAlways {
		t.Fatalf("DetectColor with FORCE_COLOR = %v, want ColorAlways", got)
	}
}

func TestDetectColorHonorsNoColor(t *testing.T) {
	t.Setenv("FORCE_COLOR", "")
	t.Setenv("NO_COLOR", "1")

	if got := DetectColor(&strings.Builder{}); got != ColorNever {
		t.Fatalf("DetectColor with NO_COLOR = %v, want ColorNever", got)
	}
}

func TestDetectColorNonTerminalIsNever(t *testing.T) {
	t.Setenv("FORCE_COLOR", "")
	t.Setenv("NO_COLOR", "")

	// A strings.Builder is not an *os.File, so it is never a terminal.
	if got := DetectColor(&strings.Builder{}); got != ColorNever {
		t.Fatalf("DetectColor for non-tty = %v, want ColorNever", got)
	}
}

func TestPrinterPlainRendering(t *testing.T) {
	var buf strings.Builder

	p := NewPrinter(&buf, ColorNever)

	p.Section("Running Go formatting")
	p.Detail("fmtkit", "Formatted 2 file(s).")
	p.SuccessDetail("status", "done")
	p.Failure("Running Go formatting failed")

	want := "\n==> Running Go formatting\n" +
		"    fmtkit       Formatted 2 file(s).\n" +
		"    status       done\n" +
		"\n!! Running Go formatting failed\n"

	if buf.String() != want {
		t.Fatalf("plain rendering mismatch\n--- got ---\n%q\n--- want ---\n%q", buf.String(), want)
	}
}

func TestPrinterColorRendering(t *testing.T) {
	var buf strings.Builder

	p := NewPrinter(&buf, ColorAlways)

	p.Section("Formatting complete")

	got := buf.String()

	for _, want := range []string{"\033[36m", "\033[1m", "\033[0m", "Formatting complete"} {
		if !strings.Contains(got, want) {
			t.Fatalf("color section missing %q:\n%q", want, got)
		}
	}
}

func TestPrinterColorAutoRendersPlain(t *testing.T) {
	var buf strings.Builder

	NewPrinter(&buf, ColorAuto).Detail("label", "value")

	if strings.Contains(buf.String(), "\033[") {
		t.Fatalf("ColorAuto emitted ANSI escapes: %q", buf.String())
	}
}

func TestStreamIndentsAndFlushesPartialLine(t *testing.T) {
	var buf strings.Builder

	p := NewPrinter(&buf, ColorNever)

	stream := p.Stream()

	_, _ = stream.Write([]byte("first line\nsecond "))
	_, _ = stream.Write([]byte("half\ntrailing"))

	if err := stream.Close(); err != nil {
		t.Fatalf("Close: %v", err)
	}

	want := "    first line\n" +
		"    second half\n" +
		"    trailing\n"

	if buf.String() != want {
		t.Fatalf("stream mismatch\n--- got ---\n%q\n--- want ---\n%q", buf.String(), want)
	}
}

package command

import (
	"bytes"
	"context"
	"testing"
)

// fixtureSet builds a Set with a run-recording command, one aliased command,
// and the given error exit code.
func fixtureSet(errExit int, stderr *bytes.Buffer, ran *string) Set {
	record := func(name string) func(context.Context, []string) int {
		return func(_ context.Context, args []string) int {
			*ran = name

			return len(args)
		}
	}

	return Set{
		Name:    "tool",
		Header:  "usage: tool <do|ping>\n",
		ErrExit: errExit,
		Stderr:  stderr,
		Commands: []Command{
			{Name: "do", Usage: "  do    do the thing\n", Run: record("do")},
			{
				Name:    "ping",
				Aliases: []string{"--ping", "-p"},
				Usage:   "  ping  ping the thing\n",
				Run:     record("ping"),
			},
			{Name: "version", Aliases: []string{"--version"}, Run: record("version")},
		},
	}
}

func TestDispatchRoutesToCommand(t *testing.T) {
	var stderr bytes.Buffer

	var ran string

	set := fixtureSet(2, &stderr, &ran)

	if code := set.Dispatch(context.Background(), []string{"do", "a", "b"}); code != 2 {
		t.Fatalf("Run should receive 2 args, got exit %d", code)
	}

	if ran != "do" {
		t.Fatalf("expected do to run, got %q", ran)
	}

	if stderr.Len() != 0 {
		t.Fatalf("unexpected stderr: %q", stderr.String())
	}
}

func TestDispatchMatchesAliases(t *testing.T) {
	for _, alias := range []string{"ping", "--ping", "-p"} {
		var stderr bytes.Buffer

		var ran string

		set := fixtureSet(1, &stderr, &ran)

		if code := set.Dispatch(context.Background(), []string{alias}); code != 0 {
			t.Fatalf("alias %q: unexpected exit %d", alias, code)
		}

		if ran != "ping" {
			t.Fatalf("alias %q did not route to ping, ran %q", alias, ran)
		}
	}
}

func TestDispatchEmptyPrintsUsageAndErrExit(t *testing.T) {
	var stderr bytes.Buffer

	var ran string

	set := fixtureSet(2, &stderr, &ran)

	if code := set.Dispatch(context.Background(), nil); code != 2 {
		t.Fatalf("empty args exit = %d, want ErrExit 2", code)
	}

	want := "usage: tool <do|ping>\n  do    do the thing\n  ping  ping the thing\n"

	if stderr.String() != want {
		t.Fatalf("usage mismatch\n got: %q\nwant: %q", stderr.String(), want)
	}
}

func TestDispatchUnknownPrintsErrorThenUsage(t *testing.T) {
	var stderr bytes.Buffer

	var ran string

	set := fixtureSet(1, &stderr, &ran)

	if code := set.Dispatch(context.Background(), []string{"bogus"}); code != 1 {
		t.Fatalf("unknown exit = %d, want ErrExit 1", code)
	}

	want := "unknown subcommand - {\"bogus\"}\n\nusage: tool <do|ping>\n  do    do the thing\n  ping  ping the thing\n"

	if stderr.String() != want {
		t.Fatalf("unknown output mismatch\n got: %q\nwant: %q", stderr.String(), want)
	}

	if ran != "" {
		t.Fatalf("no command should have run, ran %q", ran)
	}
}

func TestDispatchHelpPrintsUsageAndExitsZero(t *testing.T) {
	for _, arg := range []string{"help", "--help", "-h"} {
		var stderr bytes.Buffer

		var ran string

		set := fixtureSet(2, &stderr, &ran)

		if code := set.Dispatch(context.Background(), []string{arg}); code != 0 {
			t.Fatalf("help arg %q: exit = %d, want 0", arg, code)
		}

		if ran != "" {
			t.Fatalf("help must not run a command, ran %q", ran)
		}

		if stderr.Len() == 0 {
			t.Fatalf("help arg %q printed no usage", arg)
		}
	}
}

func TestPrintUsageComposesHeaderAndCommands(t *testing.T) {
	var out bytes.Buffer

	var ran string

	set := fixtureSet(2, &bytes.Buffer{}, &ran)

	set.PrintUsage(&out)

	want := "usage: tool <do|ping>\n  do    do the thing\n  ping  ping the thing\n"

	if out.String() != want {
		t.Fatalf("PrintUsage mismatch\n got: %q\nwant: %q", out.String(), want)
	}
}

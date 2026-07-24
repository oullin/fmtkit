// Package command is the CLI dispatch table shared by both fmtkit binaries.
// A Set is a named group of Commands with its own usage text and error exit
// code, so the two binaries' deliberate divergences (the umbrella exits 2 on a
// bad subcommand and prefixes its Go usage with "fmtkit go", the standalone
// fmtkit-go exits 1 and prefixes with "fmtkit") live in Set fields rather than
// in branching code.
package command

import (
	"context"
	"fmt"
	"io"
)

// Command is one dispatchable subcommand. Name is how it is invoked; Aliases
// are equivalent spellings (e.g. --version). Usage is this command's line(s) in
// the parent Set's usage text. Run receives the arguments after the command
// name and returns the process exit code.
type Command struct {
	Name    string
	Aliases []string
	Usage   string
	Run     func(ctx context.Context, args []string) int
}

// Set is a named group of Commands. Header prefixes the usage text; ErrExit is
// the exit code for an empty or unknown subcommand; Stderr is where usage and
// errors are written.
type Set struct {
	Name     string
	Header   string
	Commands []Command
	ErrExit  int
	Stderr   io.Writer
}

func (c Command) matches(name string) bool {
	if name == c.Name {
		return true
	}

	for _, alias := range c.Aliases {
		if name == alias {
			return true
		}
	}

	return false
}

// Dispatch routes args to a command. An empty argument list or an unknown
// subcommand prints the usage and returns ErrExit; help (help/--help/-h) prints
// the usage and returns 0; otherwise the matching command runs.
func (s Set) Dispatch(ctx context.Context, args []string) int {
	if len(args) == 0 {
		s.PrintUsage(s.Stderr)

		return s.ErrExit
	}

	name := args[0]
	rest := args[1:]

	if isHelp(name) {
		s.PrintUsage(s.Stderr)

		return 0
	}

	for _, command := range s.Commands {
		if command.matches(name) {
			return command.Run(ctx, rest)
		}
	}

	_, _ = fmt.Fprintf(s.Stderr, "unknown subcommand - {%q}\n\n", name)

	s.PrintUsage(s.Stderr)

	return s.ErrExit
}

// PrintUsage writes the Set's Header followed by each command's Usage line.
func (s Set) PrintUsage(w io.Writer) {
	_, _ = io.WriteString(w, s.Header)

	for _, command := range s.Commands {
		_, _ = io.WriteString(w, command.Usage)
	}
}

func isHelp(name string) bool {
	switch name {
	case "help", "--help", "-h":
		return true
	default:
		return false
	}
}

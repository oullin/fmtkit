package app

import (
	"fmt"
	"io"
)

func printUsage(w io.Writer) {
	_, _ = fmt.Fprintf(w, "usage: fmtkit <format|format-all|go|ts|lint|check|version|help> [args...]\n")
	_, _ = fmt.Fprintf(w, "  format [--ts] [--go] [--quiet] [paths...]  format changed files (vs HEAD) and untracked ones\n")
	_, _ = fmt.Fprintf(w, "  format-all [--ts] [--go] [--quiet]       format every file, against .\n")
	_, _ = fmt.Fprintf(w, "      --ts   only TS/Vue formatting + lint; --go   only Go formatting; default: all\n")
	_, _ = fmt.Fprintf(w, "  go <check|format|sources|version|help>  run the Go formatter CLI\n")
	_, _ = fmt.Fprintf(w, "  ts [paths...]                            run TS/Vue formatting support and oxfmt\n")
	_, _ = fmt.Fprintf(w, "  lint [paths...]                          lint TS/Vue files with oxlint\n")
	_, _ = fmt.Fprintf(w, "  check [args...]                          run the Go formatter in check mode\n")
	_, _ = fmt.Fprintf(w, "  version                                  print the fmtkit version\n")
}

func printGoUsage(w io.Writer) {
	_, _ = fmt.Fprintf(w, "fmtkit go check [paths...]\n\n")
	_, _ = fmt.Fprintf(w, "fmtkit go format [paths...]\n\n")
	_, _ = fmt.Fprintf(w, "fmtkit go sources [--include-declarations] [paths...]\n\n")
}

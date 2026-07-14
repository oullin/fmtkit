package runtimex_test

import "github.com/oullin/fmtkit/packages/runtimex"

var _ func() (runtimex.Runtime, error) = runtimex.Ensure

var _ interface {
	FormatTSBinary() string
	LintTSBinary() string
	Environment() []string
	ApplyGoEnvironment() func()
} = runtimex.Runtime{}

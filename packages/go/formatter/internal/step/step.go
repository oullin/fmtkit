package step

import (
	"go/format"

	"go.ollin.sh/fmtkit/formatter/engine"
	"golang.org/x/tools/imports"
)

type gofmtFormatter struct{}

type goimportsFormatter struct{}

func NewGofmt() engine.Formatter {
	return gofmtFormatter{}
}

func (gofmtFormatter) Name() string {
	return "gofmt"
}

func (gofmtFormatter) Format(src []byte) ([]byte, error) {
	return format.Source(src)
}

func NewGoimports() engine.Formatter {
	return goimportsFormatter{}
}

func (goimportsFormatter) Name() string {
	return "goimports"
}

func (goimportsFormatter) Format(src []byte) ([]byte, error) {
	return imports.Process("", src, nil)
}

module github.com/oullin/fmtkit/packages/formatter

go 1.26.4

require (
	github.com/oullin/fmtkit/packages/driver v0.0.0
	golang.org/x/tools v0.43.0
)

replace github.com/oullin/fmtkit/packages/driver v0.0.0 => ../driver

require (
	golang.org/x/mod v0.34.0 // indirect
	golang.org/x/sync v0.20.0 // indirect
)

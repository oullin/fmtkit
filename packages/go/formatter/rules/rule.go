package rules

// Violation reports a single rule violation.
type Violation struct {
	Rule    string `json:"rule"`
	File    string `json:"file"`
	Line    int    `json:"line,omitempty"`
	Message string `json:"message"`
}

// Rule transforms Go source and reports any violations it finds.
type Rule interface {
	Name() string
	Apply(path string, src []byte) ([]Violation, []byte, error)
}

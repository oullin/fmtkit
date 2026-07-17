package cli

type Mode string

const (
	CheckMode  Mode = "check"
	FormatMode Mode = "format"
)

func (m Mode) String() string {
	return string(m)
}

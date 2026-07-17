package cli

import "testing"

func TestModeString(t *testing.T) {
	if got := CheckMode.String(); got != "check" {
		t.Fatalf("CheckMode.String() = %q", got)
	}

	if got := FormatMode.String(); got != "format" {
		t.Fatalf("FormatMode.String() = %q", got)
	}
}

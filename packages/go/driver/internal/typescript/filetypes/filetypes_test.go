package filetypes

import "testing"

func TestFormattable(t *testing.T) {
	cases := []struct {
		name                string
		path                string
		includeDeclarations bool
		want                bool
	}{
		{"typescript is formattable", "src/app.ts", false, true},
		{"tsx is formattable", "src/Screen.tsx", false, true},
		{"mts is formattable", "src/app.mts", false, true},
		{"cts is formattable", "src/app.cts", false, true},
		{"vue is formattable", "src/component.vue", false, true},
		{"html is formattable", "src/index.html", false, true},
		{"htm is formattable", "src/index.htm", false, true},
		{"markdown md is formattable", "docs/notes.md", false, true},
		{"markdown long form is formattable", "docs/readme.markdown", false, true},
		{"unknown extensions are skipped", "src/app.go", false, false},
		{"javascript is not formattable", "src/app.js", false, false},
		{"declarations drop by default", "src/types.d.ts", false, false},
		{"declarations kept when requested", "src/types.d.ts", true, true},
		{"mts declarations drop by default", "src/types.d.mts", false, false},
		{"cts declarations drop by default", "src/types.d.cts", false, false},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			f := Filter{IncludeDeclarations: tc.includeDeclarations}

			if got := f.Formattable(tc.path); got != tc.want {
				t.Fatalf("Formattable(%q) = %v, want %v", tc.path, got, tc.want)
			}
		})
	}
}

func TestLintable(t *testing.T) {
	cases := []struct {
		name                string
		path                string
		includeDeclarations bool
		want                bool
	}{
		{"typescript is lintable", "src/app.ts", false, true},
		{"tsx is lintable", "src/Screen.tsx", false, true},
		{"mts is lintable", "src/app.mts", false, true},
		{"cts is lintable", "src/app.cts", false, true},
		{"vue is lintable", "src/component.vue", false, true},
		{"html is not lintable", "src/index.html", false, false},
		{"markdown is not lintable", "docs/notes.md", false, false},
		{"unknown extensions are skipped", "src/app.go", false, false},
		{"declarations drop by default", "src/types.d.ts", false, false},
		{"declarations kept when requested", "src/types.d.ts", true, true},
		{"mts declarations drop by default", "src/types.d.mts", false, false},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			f := Filter{IncludeDeclarations: tc.includeDeclarations}

			if got := f.Lintable(tc.path); got != tc.want {
				t.Fatalf("Lintable(%q) = %v, want %v", tc.path, got, tc.want)
			}
		})
	}
}

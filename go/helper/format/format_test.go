package format_test

import (
	"fmt"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/go/helper/format"
	"go.ollin.sh/fmtkit/go/helper/proto"
)

var allSteps = proto.Steps{Spacing: true, Gofmt: true, Goimports: true, Complexity: true}

func process(src string, steps proto.Steps) proto.Reply {
	req := proto.Request{Rel: "pkg/sample.go", Abs: "/repo/pkg/sample.go", Source: []byte(src), Steps: steps}

	return format.Process(req)
}

func TestProcessRunsStepsInOrderAndNamesTheOnesThatChanged(t *testing.T) {
	reply := process("package sample\nfunc run( ) {\n\tdefer println(\"done\")\n\treturn\n}\n", allSteps)

	if reply.Error != "" {
		t.Fatalf("error: %s", reply.Error)
	}

	if !slices.Equal(reply.Applied, []string{"spacing", "gofmt"}) {
		t.Fatalf("applied = %v", reply.Applied)
	}

	want := "package sample\n\nfunc run() {\n\tdefer println(\"done\")\n\n\treturn\n}\n"

	if string(reply.Output) != want {
		t.Fatalf("output:\n%s", reply.Output)
	}

	found := reply.Violations

	if len(found) != 1 || found[0].Rule != "spacing" || found[0].Line != 4 || found[0].Column != 0 {
		t.Fatalf("violations = %#v", found)
	}
}

func TestProcessLeavesFormattedSourceAlone(t *testing.T) {
	src := "package sample\n\nimport \"embed\"\n\n//go:embed foo.txt\nvar rootTemplateFS embed.FS\n\ntype runtime struct{}\n"
	reply := process(src, allSteps)

	if reply.Error != "" || len(reply.Applied) != 0 || len(reply.Violations) != 0 || string(reply.Output) != src {
		t.Fatalf("reply = %#v", reply)
	}
}

func TestProcessRepairsGoEmbedDirectivePlacement(t *testing.T) {
	for _, directive := range []string{"//go:embed foo.txt", "//go:embed\tfoo.txt"} {
		for _, src := range []string{
			"package sample\n\nimport \"embed\"\n\n" + directive + "\n\ntype runtime struct{}\n\nvar rootTemplateFS embed.FS\n",
			"package sample\n\n" + directive + "\n\nimport \"embed\"\n\ntype runtime struct{}\n\nvar rootTemplateFS embed.FS\n",
		} {
			reply := process(src, allSteps)
			want := "package sample\n\nimport \"embed\"\n\n" + directive + "\nvar rootTemplateFS embed.FS\n\ntype runtime struct{}\n"

			if reply.Error != "" || string(reply.Output) != want {
				t.Fatalf("reply for %q = %q, %s", src, reply.Output, reply.Error)
			}
		}
	}
}

func TestProcessSupportsGo127Syntax(t *testing.T) {
	src := "package sample\n\ntype holder struct{}\n\nfunc (holder) Echo[T any](value T) T {return value}\n\ntype embedded struct {value string}\ntype record struct {embedded}\n\nvar _ = record{value:\"hello\"}\n"
	reply := process(src, allSteps)

	if reply.Error != "" {
		t.Fatalf("error: %s", reply.Error)
	}

	for _, syntax := range []string{"func (holder) Echo[T any](value T) T", `record{value: "hello"}`} {
		if !strings.Contains(string(reply.Output), syntax) {
			t.Fatalf("Go 1.27 syntax was not preserved: %s", reply.Output)
		}
	}

	again := process(string(reply.Output), allSteps)

	if len(again.Applied) != 0 || string(again.Output) != string(reply.Output) {
		t.Fatalf("output is not stable: %#v", again)
	}
}

func TestProcessReportsTheFailingStep(t *testing.T) {
	src := "package sample\nfunc run("

	cases := map[string]proto.Steps{
		"spacing: ":    {Spacing: true, Gofmt: true},
		"gofmt: ":      {Gofmt: true},
		"goimports: ":  {Goimports: true},
		"complexity: ": {Complexity: true},
	}

	for prefix, steps := range cases {
		reply := process(src, steps)

		if !strings.HasPrefix(reply.Error, prefix) {
			t.Fatalf("error = %q, want prefix %q", reply.Error, prefix)
		}

		if string(reply.Output) != src || reply.Complexity != nil {
			t.Fatalf("a failed reply must carry the original source and no scores: %#v", reply)
		}
	}
}

func TestProcessNamesSpacingParseErrorsByRelativePath(t *testing.T) {
	reply := process("package sample\nfunc run(", proto.Steps{Spacing: true})

	if !strings.HasPrefix(reply.Error, "spacing: pkg/sample.go:") {
		t.Fatalf("error = %q", reply.Error)
	}
}

func TestGoimportsIsFormatOnlyByDefault(t *testing.T) {
	src := "package sample\n\nimport (\n\t\"strings\"\n\t\"fmt\"\n)\n\nfunc run() { fmt.Println(strings.TrimSpace(os.Args[0])) }\n"
	reply := process(src, proto.Steps{Goimports: true})

	if reply.Error != "" {
		t.Fatalf("error: %s", reply.Error)
	}

	got := string(reply.Output)

	if strings.Contains(got, "\"os\"") {
		t.Fatalf("format-only goimports added an import:\n%s", got)
	}

	if !strings.Contains(got, "import (\n\t\"fmt\"\n\t\"strings\"\n)") {
		t.Fatalf("format-only goimports did not sort the imports:\n%s", got)
	}

	if !slices.Equal(reply.Applied, []string{"goimports"}) {
		t.Fatalf("applied = %v", reply.Applied)
	}
}

func TestGoimportsResolvesImportsWhenAsked(t *testing.T) {
	abs := filepath.Join(t.TempDir(), "sample.go")
	src := "package sample\n\nfunc run(){fmt.Println(strings.TrimSpace(\" ok \"))}\n"
	reply := format.Process(proto.Request{
		Rel:    "sample.go",
		Abs:    abs,
		Source: []byte(src),
		Steps:  proto.Steps{Goimports: true, ResolveImports: true},
	})

	if reply.Error != "" {
		t.Fatalf("error: %s", reply.Error)
	}

	got := string(reply.Output)

	for _, want := range []string{"\"fmt\"", "\"strings\"", "fmt.Println(strings.TrimSpace(\" ok \"))"} {
		if !strings.Contains(got, want) {
			t.Fatalf("expected %q in:\n%s", want, got)
		}
	}
}

func TestProcessScoresTheFinalText(t *testing.T) {
	src := "package sample\nfunc run(a int) int {\n\tif a > 0 {\n\t\treturn 1\n\t}\n\treturn 0\n}\n"
	reply := process(src, allSteps)

	if reply.Error != "" {
		t.Fatalf("error: %s", reply.Error)
	}

	want := []proto.Score{{Key: "pkg/sample.go#run", Name: "run", Line: 3, Cyclomatic: 2, Cognitive: 1}}

	if !slices.Equal(reply.Complexity, want) {
		t.Fatalf("complexity = %#v, want %#v", reply.Complexity, want)
	}
}

func TestProcessWithNoStepsEchoesTheSource(t *testing.T) {
	reply := process("not go at all", proto.Steps{})

	if reply.Error != "" || string(reply.Output) != "not go at all" || reply.Applied != nil {
		t.Fatalf("reply = %#v", reply)
	}
}

// The naming checks judge the final text, so their lines match the file
// fmtkit writes, and report into Lint rather than Violations.
func TestProcessChecksNamesOnTheFinalText(t *testing.T) {
	steps := proto.Steps{Gofmt: true, MaxWords: 1, MaxLocalWords: 1, PackageStutter: true}
	reply := process("package sample\nfunc run( ) {}\n\n\n\nfunc SampleRunner() {}\n", steps)

	if reply.Error != "" {
		t.Fatalf("error: %s", reply.Error)
	}

	// gofmt drops two of the blank lines, so the input's line 6 is line 5.
	want := []string{"naming/max-name-words 5", "naming/no-package-stutter 5"}

	var got []string

	for _, v := range reply.Lint {
		got = append(got, fmt.Sprintf("%s %d", v.Rule, v.Line))
	}

	if !slices.Equal(got, want) || len(reply.Violations) != 0 {
		t.Fatalf("lint = %#v, violations = %#v", reply.Lint, reply.Violations)
	}

	if off := process("package sample\n\nfunc SampleRunner() {}\n", proto.Steps{Gofmt: true}); off.Lint != nil {
		t.Fatalf("naming ran while off: %#v", off.Lint)
	}

	broken := process("package sample\nfunc {", proto.Steps{MaxWords: 2})

	if !strings.HasPrefix(broken.Error, "naming: ") || broken.Lint != nil {
		t.Fatalf("broken = %#v", broken)
	}
}

func TestProcessLintsTheFinalTextAndDropsFindingsOnFailure(t *testing.T) {
	limits := proto.Limits{MaxDepth: 1, MaxNestedFuncs: 3, MaxLineLength: 40, TabWidth: 4}
	src := "package sample\nfunc run(xs []int) {\n    for range xs {\n        if true { println(xs, xs, xs, xs, xs, xs) }\n    }\n}\n"
	reply := process(src, proto.Steps{Gofmt: true, Lint: limits})

	if reply.Error != "" {
		t.Fatalf("error: %s", reply.Error)
	}

	var got []string

	for _, f := range reply.Lint {
		got = append(got, fmt.Sprintf("%s %d:%d", f.Rule, f.Line, f.Column))
	}

	// gofmt splits the if onto its own lines, so only the formatted text's
	// positions are right; its tabs count 4 columns, not the input's 8 spaces.
	want := []string{"go/max-depth 5:3", "go/line-length 6:32"}

	if !slices.Equal(got, want) {
		t.Fatalf("lint = %v, want %v\n%s", got, want, reply.Output)
	}

	if off := process(src, proto.Steps{Gofmt: true}); off.Lint != nil {
		t.Fatalf("lint with every limit off = %v", off.Lint)
	}

	failed := process("package sample\nfunc (", proto.Steps{Lint: limits})

	if !strings.HasPrefix(failed.Error, "lint: ") || failed.Lint != nil {
		t.Fatalf("reply = %#v", failed)
	}
}

// The checks share one parse of the final text: a text that does not parse
// fails the first step that needs it, and sharing changes no score.
func TestProcessParsesTheFinalTextOnceForEveryCheck(t *testing.T) {
	limits := proto.Limits{MaxDepth: 4, MaxNestedFuncs: 3, MaxLineLength: 120, TabWidth: 4}
	every := proto.Steps{MaxWords: 2, Complexity: true, Lint: limits}

	if broken := process("package sample\nfunc (", every); !strings.HasPrefix(broken.Error, "naming: ") {
		t.Fatalf("error = %q", broken.Error)
	}

	src := "package sample\n\n// run loops.\nfunc run(xs []int) {\n\tfor range xs {\n\t\tif len(xs) > 1 {\n\t\t\treturn\n\t\t}\n\t}\n}\n"
	alone := process(src, proto.Steps{Complexity: true})
	shared := process(src, every)

	if alone.Error != "" || shared.Error != "" || !slices.Equal(alone.Complexity, shared.Complexity) {
		t.Fatalf("complexity alone = %+v, with the other checks = %+v (%q)",
			alone.Complexity, shared.Complexity, shared.Error)
	}
}

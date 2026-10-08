package format_test

import (
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/go/helper/format"
	"go.ollin.sh/fmtkit/go/helper/proto"
)

var allSteps = proto.Steps{Spacing: true, Gofmt: true, Goimports: true, Complexity: true}

func process(src string, steps proto.Steps) proto.Reply {
	return format.Process(proto.Request{Rel: "pkg/sample.go", Abs: "/repo/pkg/sample.go", Source: []byte(src), Steps: steps})
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

	if len(reply.Violations) != 1 || reply.Violations[0].Rule != "spacing" || reply.Violations[0].Line != 4 || reply.Violations[0].Column != 0 {
		t.Fatalf("violations = %#v", reply.Violations)
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

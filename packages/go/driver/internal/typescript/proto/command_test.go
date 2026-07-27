package proto

import (
	"reflect"
	"testing"
)

func TestPipelineCommandArgv(t *testing.T) {
	cmd := PipelineCommand{
		OxfmtBin:    "/tools/fmtkit-ts-sidecar",
		OxfmtConfig: "/cfg/.oxfmtrc.json",
		FormatFiles: []string{"/work/app.ts", "/work/types.ts"},
		SyntaxFiles: []string{"/work/app.ts", "/work/decl.d.ts", "/work/types.ts"},
	}

	want := []string{
		"pipeline",
		"--oxfmt-bin", "/tools/fmtkit-ts-sidecar",
		"--oxfmt-config", "/cfg/.oxfmtrc.json",
		"--format-files",
		"/work/app.ts", "/work/types.ts",
		"--syntax-files",
		"/work/app.ts", "/work/decl.d.ts", "/work/types.ts",
	}

	if got := cmd.Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

func TestPipelineCommandArgvOmitsEmptyConfig(t *testing.T) {
	cmd := PipelineCommand{
		OxfmtBin:    "/tools/fmtkit-ts-sidecar",
		FormatFiles: []string{"/work/app.ts"},
		SyntaxFiles: []string{"/work/app.ts"},
	}

	want := []string{
		"pipeline",
		"--oxfmt-bin", "/tools/fmtkit-ts-sidecar",
		"--format-files",
		"/work/app.ts",
		"--syntax-files",
		"/work/app.ts",
	}

	if got := cmd.Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

func TestPipelineCommandArgvAlwaysCarriesFileSentinels(t *testing.T) {
	// Even with no files, the --format-files/--syntax-files markers are present
	// so the sidecar's parser sees empty lists rather than a missing section.
	cmd := PipelineCommand{OxfmtBin: "sidecar"}

	want := []string{
		"pipeline",
		"--oxfmt-bin", "sidecar",
		"--format-files",
		"--syntax-files",
	}

	if got := cmd.Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

func TestOxlintCommandArgvViaSidecar(t *testing.T) {
	cmd := OxlintCommand{
		ViaSidecar: true,
		Config:     "/cfg/.oxlintrc.json",
		Files:      []string{"/work/app.ts"},
	}

	want := []string{
		"oxlint",
		"--config", "/cfg/.oxlintrc.json",
		"/work/app.ts",
	}

	if got := cmd.Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

func TestOxlintCommandArgvWithFix(t *testing.T) {
	cmd := OxlintCommand{
		ViaSidecar: true,
		Fix:        true,
		Config:     "/cfg/.oxlintrc.json",
		Files:      []string{"/work/app.ts"},
	}

	want := []string{
		"oxlint",
		"--fix",
		"--config", "/cfg/.oxlintrc.json",
		"/work/app.ts",
	}

	if got := cmd.Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

func TestOxlintCommandArgvDirectBinOmitsMode(t *testing.T) {
	// A direct OXLINT_BIN override runs oxlint without the sidecar's mode word.
	cmd := OxlintCommand{
		ViaSidecar: false,
		Files:      []string{"/work/app.ts"},
	}

	want := []string{"/work/app.ts"}

	if got := cmd.Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

func TestMigrateCommandArgvViaSidecar(t *testing.T) {
	want := []string{"oxfmt", "--migrate=prettier"}

	if got := (MigrateCommand{ViaSidecar: true}).Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

func TestMigrateCommandArgvDirectBin(t *testing.T) {
	want := []string{"--migrate=prettier"}

	if got := (MigrateCommand{ViaSidecar: false}).Argv(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Argv() = %q, want %q", got, want)
	}
}

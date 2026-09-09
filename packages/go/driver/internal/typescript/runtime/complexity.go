package runtime

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"go.ollin.sh/fmtkit/complexity"
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/typescript/proto"
	"go.ollin.sh/fmtkit/driver/internal/typescript/sourcefiles"
)

// sidecarScan is the JSON the sidecar's complexity mode writes: the numbers
// only. The limits, the allow list, and the exit policy stay in the driver so
// both language lanes are judged by one implementation of the rules.
type sidecarScan struct {
	Functions []complexity.Function    `json:"functions"`
	Errors    []complexity.ErrorResult `json:"errors"`
}

// RunComplexity scores every TS/JS source under the request's scopes through
// the sidecar and returns the lane's measurement.
//
// The file list travels as a NUL-separated listing rather than an argv tail: a
// whole repository's worth of paths does not fit in one command line.
func (i Invoker) RunComplexity(ctx context.Context, req Request) (complexity.Scan, error) {
	cwd, err := i.sourcesCwd()

	if err != nil {
		return complexity.Scan{}, err
	}

	files, warnings, err := collectScorable(ctx, cwd, req.Scopes, false, req.Selection)

	if err != nil {
		return complexity.Scan{}, err
	}

	for _, warning := range warnings {
		_, _ = fmt.Fprintf(req.Stderr, "[sources] %s\n", warning)
	}

	scan := complexity.Scan{Lane: complexity.LaneTS, Files: relativeFiles(cwd, files)}

	if len(files) == 0 {
		return scan, nil
	}

	listing, cleanup, err := writeListing(files)

	if err != nil {
		return complexity.Scan{}, err
	}

	defer cleanup()

	output, err := i.capture(ctx, i.complexityExecutable(), proto.ComplexityCommand{Root: cwd, FilesFrom: listing}.Argv(), req)

	if err != nil && len(output) == 0 {
		return complexity.Scan{}, err
	}

	var decoded sidecarScan

	if decodeErr := json.Unmarshal(output, &decoded); decodeErr != nil {
		return complexity.Scan{}, fmt.Errorf("decode complexity scan: %w", decodeErr)
	}

	scan.Functions = decoded.Functions
	scan.Errors = decoded.Errors

	return scan, nil
}

// complexityExecutable resolves the executable spawned for the scan: the same
// FMTKIT_TS_PIPELINE_BIN override the pipeline honours, otherwise the sidecar.
func (i Invoker) complexityExecutable() string {
	return i.pipelineExecutable()
}

// capture spawns the sidecar with its stdout collected rather than streamed,
// because this mode's stdout is the report itself.
func (i Invoker) capture(ctx context.Context, bin string, args []string, req Request) ([]byte, error) {
	cmd := exec.CommandContext(ctx, bin, args...)

	var out strings.Builder

	cmd.Stdout = &out
	cmd.Stderr = req.Stderr
	cmd.Env = append(os.Environ(),
		"GIT_CONFIG_COUNT=1",
		"GIT_CONFIG_KEY_0=safe.directory",
		"GIT_CONFIG_VALUE_0=*",
	)

	err := cmd.Run()

	return []byte(out.String()), err
}

// writeListing writes the NUL-separated file list the sidecar reads, returning
// its path and the cleanup that removes it.
func writeListing(files []string) (string, func(), error) {
	file, err := os.CreateTemp("", "fmtkit-complexity-*.list")

	if err != nil {
		return "", func() {}, fmt.Errorf("stage complexity file list: %w", err)
	}

	cleanup := func() { _ = os.Remove(file.Name()) }

	if _, err := file.WriteString(strings.Join(files, "\x00")); err != nil {
		_ = file.Close()

		cleanup()

		return "", func() {}, fmt.Errorf("stage complexity file list: %w", err)
	}

	if err := file.Close(); err != nil {
		cleanup()

		return "", func() {}, fmt.Errorf("stage complexity file list: %w", err)
	}

	return file.Name(), cleanup, nil
}

// relativeFiles renders absolute paths relative to cwd in slash form, matching
// the keys the sidecar reports.
func relativeFiles(cwd string, files []string) []string {
	out := make([]string, 0, len(files))

	for _, file := range files {
		relative, err := filepath.Rel(cwd, file)

		if err != nil {
			relative = file
		}

		out = append(out, filepath.ToSlash(relative))
	}

	return out
}

func collectScorable(ctx context.Context, cwd string, scopes []string, includeDeclarations bool, selection gitfiles.Selection) ([]string, []string, error) {
	collector, err := sourcefiles.New(cwd, selection, includeDeclarations)

	if err != nil {
		return nil, nil, err
	}

	return collector.Scorable(ctx, scopes)
}

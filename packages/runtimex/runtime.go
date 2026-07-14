package runtimex

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/go-playground/validator/v10"
)

type Runtime struct {
	binDir string
	root   string
	self   string
}

type runtimeInput struct {
	RuntimeDir string `validate:"omitempty,absolutepath"`
}

const runtimeVersion = "v2"

func Ensure() (Runtime, error) {
	self, err := os.Executable()

	if err != nil {
		return Runtime{}, fmt.Errorf("resolve executable: %w", err)
	}

	payload, ok, err := bundledRuntimePayload()

	if err != nil {
		return Runtime{}, err
	}

	root, err := runtimeRoot(payload, ok)

	if err != nil {
		return Runtime{}, err
	}

	if err := ensureRuntimeCache(root, payload, ok); err != nil {
		return Runtime{}, err
	}

	binDir := filepath.Join(root, "bin")

	if err := ensureSourceShim(root, self); err != nil {
		return Runtime{}, err
	}

	return Runtime{binDir: binDir, root: root, self: self}, nil
}

func runtimeRoot(payload runtimePayload, hasPayload bool) (string, error) {
	input, err := normalizeRuntimeInput(runtimeInput{RuntimeDir: os.Getenv("GO_FMT_RUNTIME_DIR")})

	if err != nil {
		return "", err
	}

	base := input.RuntimeDir

	if base != "" {
		if !filepath.IsAbs(base) {
			return "", fmt.Errorf("GO_FMT_RUNTIME_DIR must be an absolute path")
		}
	} else {
		cacheRoot, err := os.UserCacheDir()

		if err != nil {
			return "", fmt.Errorf("resolve user cache dir: %w", err)
		}

		base = filepath.Join(cacheRoot, "go-fmt", "contained")
	}

	base = filepath.Clean(base)

	if err := ensureTrustedBase(base); err != nil {
		return "", err
	}

	identity := "unbundled"

	if hasPayload {
		identity = payload.manifest.ArchiveSHA256
	}

	return filepath.Join(base, "runtime", runtimeVersion, runtime.GOOS+"-"+runtime.GOARCH, identity), nil
}

func normalizeRuntimeInput(input runtimeInput) (runtimeInput, error) {
	if strings.TrimSpace(input.RuntimeDir) == "" {
		input.RuntimeDir = ""
	}

	validate := validator.New()

	if err := validate.RegisterValidation("absolutepath", func(level validator.FieldLevel) bool {
		return filepath.IsAbs(level.Field().String())
	}); err != nil {
		return runtimeInput{}, fmt.Errorf("configure runtime input validation: %w", err)
	}

	if err := validate.Struct(input); err != nil {
		return runtimeInput{}, fmt.Errorf("GO_FMT_RUNTIME_DIR must be an absolute path: %w", err)
	}

	return input, nil
}

func (r Runtime) FormatTSBinary() string {
	if value := strings.TrimSpace(os.Getenv("FORMAT_TS_BIN")); value != "" {
		return value
	}

	return filepath.Join(r.binDir, "fmt-ts")
}

func (r Runtime) LintTSBinary() string {
	if value := strings.TrimSpace(os.Getenv("FORMAT_LINT_BIN")); value != "" {
		return value
	}

	return filepath.Join(r.binDir, "fmt-lint")
}

func (r Runtime) Environment() []string {
	env := os.Environ()
	env = append(env,
		"FMTKIT_SOURCES_BIN="+filepath.Join(r.binDir, "fmt-sources"),
		"GO_FMT_SOURCES_BIN="+filepath.Join(r.binDir, "fmt-sources"),
	)

	return env
}

func (r Runtime) ApplyGoEnvironment() func() {
	updates := map[string]string{}
	goRoot := filepath.Join(r.root, "go")
	goBin := filepath.Join(goRoot, "bin", "go")

	if _, err := os.Stat(goBin); err == nil {
		updates = map[string]string{
			"PATH":       r.binDir + string(os.PathListSeparator) + os.Getenv("PATH"),
			"GOROOT":     goRoot,
			"GOCACHE":    filepath.Join(r.root, "cache", "go-build"),
			"GOPATH":     filepath.Join(r.root, "cache", "gopath"),
			"GOMODCACHE": filepath.Join(r.root, "cache", "gopath", "pkg", "mod"),
		}
	}

	previous := map[string]*string{}

	for key, value := range updates {
		if current, ok := os.LookupEnv(key); ok {
			copy := current
			previous[key] = &copy
		} else {
			previous[key] = nil
		}

		_ = os.Setenv(key, value)
	}

	return func() {
		for key, value := range previous {
			if value == nil {
				_ = os.Unsetenv(key)

				continue
			}

			_ = os.Setenv(key, *value)
		}
	}
}

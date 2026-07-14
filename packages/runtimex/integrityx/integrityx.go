package integrityx

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"strings"
	"unicode"

	"github.com/go-playground/validator/v10"
)

type Manifest struct {
	ArchiveSHA256 string   `json:"archive_sha256" validate:"notblank,sha256hex"`
	GOOS          string   `json:"goos" validate:"notblank,oneof=darwin linux"`
	GOARCH        string   `json:"goarch" validate:"notblank,oneof=amd64 arm64"`
	TreeSHA256    string   `json:"tree_sha256" validate:"notblank,sha256hex"`
	Required      []string `json:"required" validate:"min=1,dive,canonicalpath"`
}

func Build(root, archive, goos, goarch string, required []string) (Manifest, error) {
	required, err := normalizeRequired(required)

	if err != nil {
		return Manifest{}, err
	}

	tree, err := TreeSHA256(root, nil)

	if err != nil {
		return Manifest{}, err
	}

	archiveHash, err := FileSHA256(archive)

	if err != nil {
		return Manifest{}, err
	}

	manifest := Manifest{ArchiveSHA256: archiveHash, GOOS: goos, GOARCH: goarch, TreeSHA256: tree, Required: required}

	return normalizeAndValidateManifest(manifest)
}

func Parse(content []byte) (Manifest, error) {
	var manifest Manifest

	if err := json.Unmarshal(content, &manifest); err != nil {
		return Manifest{}, fmt.Errorf("parse runtime manifest: %w", err)
	}

	return normalizeAndValidateManifest(manifest)
}

func Marshal(manifest Manifest) ([]byte, error) {
	manifest, err := normalizeAndValidateManifest(manifest)

	if err != nil {
		return nil, err
	}

	content, err := json.MarshalIndent(manifest, "", "  ")

	if err != nil {
		return nil, fmt.Errorf("marshal runtime manifest: %w", err)
	}

	return append(content, '\n'), nil
}

func ValidateArchive(content []byte, manifest Manifest) error {
	manifest, err := normalizeAndValidateManifest(manifest)

	if err != nil {
		return err
	}

	sum := sha256.Sum256(content)

	if hex.EncodeToString(sum[:]) != manifest.ArchiveSHA256 {
		return fmt.Errorf("runtime archive hash does not match manifest")
	}

	return nil
}

func ValidateTree(root string, manifest Manifest, excluded map[string]struct{}) error {
	manifest, err := normalizeAndValidateManifest(manifest)

	if err != nil {
		return err
	}

	tree, err := TreeSHA256(root, excluded)

	if err != nil {
		return err
	}

	if tree != manifest.TreeSHA256 {
		return fmt.Errorf("runtime cache hash does not match manifest")
	}

	for _, required := range manifest.Required {
		info, err := os.Lstat(filepath.Join(root, filepath.FromSlash(required)))

		if err != nil {
			return fmt.Errorf("required runtime file %s: %w", required, err)
		}

		if !info.Mode().IsRegular() {
			return fmt.Errorf("required runtime file %s is not regular", required)
		}
	}

	return nil
}

// ValidatePlatform verifies that a manifest belongs to the requested supported
// platform. Callers use it at archive, release, and embedded-payload boundaries.
func ValidatePlatform(manifest Manifest, goos, goarch string) error {
	manifest, err := normalizeAndValidateManifest(manifest)

	if err != nil {
		return err
	}

	goos = strings.TrimSpace(goos)
	goarch = strings.TrimSpace(goarch)

	if manifest.GOOS != goos || manifest.GOARCH != goarch {
		return fmt.Errorf("runtime manifest platform %s/%s does not match requested platform %s/%s", manifest.GOOS, manifest.GOARCH, goos, goarch)
	}

	return nil
}

func FileSHA256(path string) (string, error) {
	file, err := os.Open(path)

	if err != nil {
		return "", err
	}

	defer func() {
		_ = file.Close()
	}()

	hash := sha256.New()

	if _, err := io.Copy(hash, file); err != nil {
		return "", err
	}

	return hex.EncodeToString(hash.Sum(nil)), nil
}

func TreeSHA256(root string, excluded map[string]struct{}) (string, error) {
	hash := sha256.New()

	err := filepath.WalkDir(root, func(path string, entry os.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}

		if path == root {
			return nil
		}

		rel, err := filepath.Rel(root, path)

		if err != nil {
			return err
		}

		rel = filepath.ToSlash(rel)

		if _, ok := excluded[rel]; ok {
			if entry.IsDir() {
				return filepath.SkipDir
			}

			return nil
		}

		info, err := entry.Info()

		if err != nil {
			return err
		}

		if info.Mode()&os.ModeSymlink != 0 {
			return fmt.Errorf("runtime tree contains symlink %s", rel)
		}

		if info.IsDir() {
			writeTreeRecord(hash, "d", rel)

			return nil
		}

		if !info.Mode().IsRegular() {
			return fmt.Errorf("runtime tree contains unsupported entry %s", rel)
		}

		fileHash, err := FileSHA256(path)

		if err != nil {
			return err
		}

		executable := "0"

		if info.Mode().Perm()&0o111 != 0 {
			executable = "1"
		}

		writeTreeRecord(hash, "f", rel, executable, fileHash)

		return nil
	})

	if err != nil {
		return "", err
	}

	return hex.EncodeToString(hash.Sum(nil)), nil
}

// ValidateRequired accepts only canonical, slash-delimited relative file paths.
// The manifest is part of the runtime trust boundary, so validate it wherever it
// enters or is consumed rather than relying on a previous validation step.
func ValidateRequired(required []string) error {
	_, err := normalizeRequired(required)

	return err
}

func normalizeAndValidateManifest(manifest Manifest) (Manifest, error) {
	manifest.ArchiveSHA256 = strings.TrimSpace(manifest.ArchiveSHA256)
	manifest.GOOS = strings.TrimSpace(manifest.GOOS)
	manifest.GOARCH = strings.TrimSpace(manifest.GOARCH)
	manifest.TreeSHA256 = strings.TrimSpace(manifest.TreeSHA256)

	var err error

	manifest.Required, err = normalizeRequired(manifest.Required)

	if err != nil {
		return Manifest{}, err
	}

	validate := validator.New()

	if err := validate.RegisterValidation("notblank", notBlank); err != nil {
		return Manifest{}, fmt.Errorf("configure runtime manifest validation: %w", err)
	}

	if err := validate.RegisterValidation("canonicalpath", canonicalPath); err != nil {
		return Manifest{}, fmt.Errorf("configure runtime manifest validation: %w", err)
	}

	if err := validate.RegisterValidation("sha256hex", sha256Hex); err != nil {
		return Manifest{}, fmt.Errorf("configure runtime manifest validation: %w", err)
	}

	validate.RegisterStructValidation(func(level validator.StructLevel) {
		manifest := level.Current().Interface().(Manifest)

		if !supportedPlatform(manifest.GOOS, manifest.GOARCH) {
			level.ReportError(manifest.GOARCH, "goarch", "GOARCH", "supportedplatform", manifest.GOOS)
		}
	}, Manifest{})

	if err := validate.Struct(manifest); err != nil {
		return Manifest{}, fmt.Errorf("invalid runtime manifest: %w", err)
	}

	return manifest, nil
}

func supportedPlatform(goos, goarch string) bool {
	switch goos + "/" + goarch {
	case "darwin/arm64", "linux/amd64", "linux/arm64":
		return true
	default:
		return false
	}
}

func normalizeRequired(required []string) ([]string, error) {
	normalized := make([]string, len(required))

	for index, value := range required {
		value = strings.TrimSpace(value)

		if value == "" || value == "." || path.IsAbs(value) || filepath.IsAbs(value) ||
			strings.ContainsAny(value, `\\:`) || strings.IndexFunc(value, unicode.IsControl) >= 0 ||
			path.Clean(value) != value || value == ".." || strings.HasPrefix(value, "../") {
			return nil, fmt.Errorf("invalid required runtime path %q", value)
		}

		normalized[index] = value
	}

	return normalized, nil
}

func notBlank(level validator.FieldLevel) bool {
	return strings.TrimSpace(level.Field().String()) != ""
}

func canonicalPath(level validator.FieldLevel) bool {
	_, err := normalizeRequired([]string{level.Field().String()})

	return err == nil
}

func sha256Hex(level validator.FieldLevel) bool {
	value := level.Field().String()

	if len(value) != sha256.Size*2 {
		return false
	}

	_, err := hex.DecodeString(value)

	return err == nil
}

func writeTreeRecord(hash io.Writer, fields ...string) {
	var length [8]byte

	binary.BigEndian.PutUint64(length[:], uint64(len(fields)))
	_, _ = hash.Write(length[:])

	for _, field := range fields {
		binary.BigEndian.PutUint64(length[:], uint64(len(field)))
		_, _ = hash.Write(length[:])
		_, _ = io.WriteString(hash, field)
	}
}

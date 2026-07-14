package runtimeintegrity

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
)

type Manifest struct {
	ArchiveSHA256 string   `json:"archive_sha256"`
	TreeSHA256    string   `json:"tree_sha256"`
	Required      []string `json:"required"`
}

func Build(root, archive string, required []string) (Manifest, error) {
	tree, err := TreeSHA256(root, nil)

	if err != nil {
		return Manifest{}, err
	}

	archiveHash, err := FileSHA256(archive)

	if err != nil {
		return Manifest{}, err
	}

	return Manifest{ArchiveSHA256: archiveHash, TreeSHA256: tree, Required: required}, nil
}

func Parse(content []byte) (Manifest, error) {
	var manifest Manifest

	if err := json.Unmarshal(content, &manifest); err != nil {
		return Manifest{}, fmt.Errorf("parse runtime manifest: %w", err)
	}

	if manifest.ArchiveSHA256 == "" || manifest.TreeSHA256 == "" || len(manifest.Required) == 0 {
		return Manifest{}, fmt.Errorf("runtime manifest is incomplete")
	}

	return manifest, nil
}

func Marshal(manifest Manifest) ([]byte, error) {
	content, err := json.MarshalIndent(manifest, "", "  ")

	if err != nil {
		return nil, fmt.Errorf("marshal runtime manifest: %w", err)
	}

	return append(content, '\n'), nil
}

func ValidateArchive(content []byte, manifest Manifest) error {
	sum := sha256.Sum256(content)

	if hex.EncodeToString(sum[:]) != manifest.ArchiveSHA256 {
		return fmt.Errorf("runtime archive hash does not match manifest")
	}

	return nil
}

func ValidateTree(root string, manifest Manifest, excluded map[string]struct{}) error {
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
			_, _ = io.WriteString(hash, "d:"+rel+"\n")

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

		_, _ = io.WriteString(hash, strings.Join([]string{"f", rel, executable, fileHash}, ":")+"\n")

		return nil
	})

	if err != nil {
		return "", err
	}

	return hex.EncodeToString(hash.Sum(nil)), nil
}

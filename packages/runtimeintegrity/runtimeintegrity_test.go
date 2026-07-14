package runtimeintegrity

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestTreeSHA256LengthDelimitsDelimiterContainingNames(t *testing.T) {
	rootA := t.TempDir()
	rootB := t.TempDir()
	content := []byte("content")
	sum := sha256.Sum256(content)

	// These two trees produced the same newline/colon-delimited record stream.
	// The canonical record encoding must keep their hashes distinct.
	name := "a\nf:b:0:" + hex.EncodeToString(sum[:])

	if err := os.Mkdir(filepath.Join(rootA, name), 0o700); err != nil {
		t.Fatalf("create delimiter-containing directory: %v", err)
	}

	if err := os.Mkdir(filepath.Join(rootB, "a"), 0o700); err != nil {
		t.Fatalf("create directory: %v", err)
	}

	if err := os.WriteFile(filepath.Join(rootB, "b"), content, 0o600); err != nil {
		t.Fatalf("write file: %v", err)
	}

	hashA, err := TreeSHA256(rootA, nil)

	if err != nil {
		t.Fatalf("hash first tree: %v", err)
	}

	hashB, err := TreeSHA256(rootB, nil)

	if err != nil {
		t.Fatalf("hash second tree: %v", err)
	}

	if hashA == hashB {
		t.Fatal("delimiter-containing names must not collide")
	}
}

func TestRequiredPathsRejectUnsafeValuesAtEveryBoundary(t *testing.T) {
	for _, required := range []string{"", ".", "../outside", "/absolute", "C:/outside", `C:\\outside`, "dir\\file", "dir/../file", "dir//file"} {
		t.Run(required, func(t *testing.T) {
			if err := ValidateRequired([]string{required}); err == nil {
				t.Fatal("expected required path rejection")
			}

			if _, err := Build("", "", []string{required}); err == nil {
				t.Fatal("expected build rejection")
			}

			content, err := json.Marshal(Manifest{ArchiveSHA256: "archive", TreeSHA256: "tree", Required: []string{required}})

			if err != nil {
				t.Fatalf("marshal manifest: %v", err)
			}

			if _, err := Parse(content); err == nil {
				t.Fatal("expected parse rejection")
			}

			if err := ValidateTree(t.TempDir(), Manifest{Required: []string{required}}, nil); err == nil {
				t.Fatal("expected use-boundary rejection")
			}
		})
	}
}

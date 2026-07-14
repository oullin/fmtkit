package runtimex

import (
	"embed"
	"fmt"
	"os"
	"runtime"

	"github.com/oullin/fmtkit/packages/runtimex/integrityx"
)

//go:embed assets/*
var runtimeAssets embed.FS

type runtimePayload struct {
	archive  []byte
	manifest integrityx.Manifest
}

func bundledRuntimePayload() (runtimePayload, bool, error) {
	archiveName := "assets/runtime-" + runtime.GOOS + "-" + runtime.GOARCH + ".tar.gz"
	archive, err := runtimeAssets.ReadFile(archiveName)

	if os.IsNotExist(err) {
		return runtimePayload{}, false, nil
	}

	if err != nil {
		return runtimePayload{}, false, fmt.Errorf("read bundled runtime %s: %w", archiveName, err)
	}

	manifestName := archiveName + ".manifest.json"
	manifestContent, err := runtimeAssets.ReadFile(manifestName)

	if err != nil {
		return runtimePayload{}, false, fmt.Errorf("read runtime manifest %s: %w", manifestName, err)
	}

	manifest, err := integrityx.Parse(manifestContent)

	if err != nil {
		return runtimePayload{}, false, err
	}

	if err := integrityx.ValidateArchive(archive, manifest); err != nil {
		return runtimePayload{}, false, err
	}

	if err := integrityx.ValidatePlatform(manifest, runtime.GOOS, runtime.GOARCH); err != nil {
		return runtimePayload{}, false, fmt.Errorf("validate runtime manifest platform for %s: %w", archiveName, err)
	}

	return runtimePayload{archive: archive, manifest: manifest}, true, nil
}

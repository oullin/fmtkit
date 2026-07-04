package engine

import (
	"fmt"
	"strings"
)

func generateDiff(old, new string) string {
	oldLines := strings.Split(old, "\n")
	newLines := strings.Split(new, "\n")

	var diff strings.Builder

	i, j := 0, 0

	for i < len(oldLines) || j < len(newLines) {
		if i < len(oldLines) && j < len(newLines) && oldLines[i] == newLines[j] {
			i++
			j++

			continue
		}

		if i < len(oldLines) && (j >= len(newLines) || oldLines[i] != newLines[j]) {
			fmt.Fprintf(&diff, "-%s\n", oldLines[i])

			i++
		}

		if j < len(newLines) {
			fmt.Fprintf(&diff, "+%s\n", newLines[j])

			j++
		}
	}

	return diff.String()
}

package report

import (
	"bytes"
	"testing"

	formatterengine "github.com/oullin/fmtkit/packages/formatter/engine"
	"github.com/oullin/fmtkit/packages/formatter/rules"
	"github.com/oullin/fmtkit/packages/vet"
)

func TestProjectReportNormalizesFormatterAndVetResults(t *testing.T) {
	projected := projectReport("/work", sampleCombinedReport())

	if projected.Result != "fail" {
		t.Fatalf("unexpected combined result: %#v", projected)
	}

	if projected.Formatter.Result != "fail" || projected.Formatter.Files != 2 || projected.Formatter.Changed != 1 || projected.Formatter.Violations != 1 {
		t.Fatalf("unexpected formatter summary: %#v", projected.Formatter)
	}

	if len(projected.Formatter.Results) != 2 {
		t.Fatalf("unexpected formatter results: %#v", projected.Formatter.Results)
	}

	changed := projected.Formatter.Results[0]

	if changed.File != "sample.go" || !changed.Changed || len(changed.Applied) != 2 || len(changed.Violations) != 1 || changed.Error != "" {
		t.Fatalf("unexpected changed result: %#v", changed)
	}

	errored := projected.Formatter.Results[1]

	if errored.File != "broken.go" || errored.Error != "parse error" {
		t.Fatalf("unexpected errored result: %#v", errored)
	}

	if len(projected.Formatter.Errors) != 2 {
		t.Fatalf("unexpected formatter errors: %#v", projected.Formatter.Errors)
	}

	if projected.Formatter.Errors[0].File != "walk.go" || projected.Formatter.Errors[1].File != "broken.go" {
		t.Fatalf("unexpected formatter error paths: %#v", projected.Formatter.Errors)
	}

	if projected.Vet.Status != "fail" || len(projected.Vet.Errors) != 1 || projected.Vet.Errors[0].File != "module-a" {
		t.Fatalf("unexpected vet projection: %#v", projected.Vet)
	}
}

func TestRenderJSONUsesProjectedReport(t *testing.T) {
	var out bytes.Buffer

	if err := RenderJSON(&out, "/work", sampleCombinedReport()); err != nil {
		t.Fatalf("render json: %v", err)
	}

	const want = "{\"result\":\"fail\",\"formatter\":{\"result\":\"fail\",\"files\":2,\"changed\":1,\"results\":[{\"file\":\"sample.go\",\"applied\":[\"spacing\",\"gofmt\"],\"violations\":[{\"rule\":\"spacing\",\"line\":7,\"message\":\"after if statement\"}],\"changed\":true}],\"errors\":[{\"file\":\"walk.go\",\"message\":\"walk failed\"},{\"file\":\"broken.go\",\"message\":\"parse error\"}]},\"vet\":{\"status\":\"fail\",\"errors\":[{\"file\":\"module-a\",\"message\":\"automatic go vet ./... failed:\\nvet output\"}]}}\n"

	if out.String() != want {
		t.Fatalf("unexpected json output:\n%s", out.String())
	}
}

func TestRenderAgentUsesProjectedReport(t *testing.T) {
	var out bytes.Buffer

	if err := RenderAgent(&out, "/work", sampleCombinedReport()); err != nil {
		t.Fatalf("render agent: %v", err)
	}

	const want = "{\n  \"result\": \"fail\",\n  \"formatter\": {\n    \"result\": \"fail\",\n    \"summary\": {\n      \"files\": 2,\n      \"changed\": 1,\n      \"violations\": 1\n    },\n    \"changed\": [\n      {\n        \"file\": \"sample.go\",\n        \"steps\": [\n          \"spacing\",\n          \"gofmt\"\n        ]\n      }\n    ],\n    \"violations\": [\n      {\n        \"file\": \"sample.go\",\n        \"rule\": \"spacing\",\n        \"line\": 7,\n        \"message\": \"after if statement\"\n      }\n    ],\n    \"errors\": [\n      {\n        \"file\": \"walk.go\",\n        \"message\": \"walk failed\"\n      },\n      {\n        \"file\": \"broken.go\",\n        \"message\": \"parse error\"\n      }\n    ]\n  },\n  \"vet\": {\n    \"status\": \"fail\",\n    \"errors\": [\n      {\n        \"file\": \"module-a\",\n        \"message\": \"automatic go vet ./... failed:\\nvet output\"\n      }\n    ]\n  }\n}\n"

	if out.String() != want {
		t.Fatalf("unexpected agent output:\n%s", out.String())
	}
}

func sampleCombinedReport() Combined {
	return Combined{
		Formatter: formatterengine.Report{
			Result:  "fail",
			Files:   2,
			Changed: 1,
			Results: []formatterengine.FileResult{
				{
					File:    "/work/sample.go",
					Applied: []string{"spacing", "gofmt"},
					Violations: []rules.Violation{
						{
							Rule:    "spacing",
							Line:    7,
							Message: "after if statement",
						},
					},
					Changed: true,
				},
				{
					File:  "/work/broken.go",
					Error: "parse error",
				},
			},
			Errors: []formatterengine.ErrorResult{
				{
					File:    "/work/walk.go",
					Message: "walk failed",
				},
			},
		},
		Vet: vet.Report{
			Root: "/work",
			Errors: []vet.ErrorResult{
				{
					File:    "/work/module-a",
					Message: "automatic go vet ./... failed:\nvet output",
				},
			},
		},
	}
}

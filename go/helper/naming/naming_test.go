package naming_test

import (
	"reflect"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/go/helper/naming"
)

// reported returns the names Check reports under rule, in source order.
func reported(t *testing.T, src string, options naming.Options, rule string) []string {
	t.Helper()

	findings, err := naming.Check("a.go", []byte(src), options)

	if err != nil {
		t.Fatal(err)
	}

	var names []string

	for _, f := range findings {
		if f.Rule == rule {
			names = append(names, strings.SplitN(f.Message, `"`, 3)[1])
		}
	}

	return names
}

func words(t *testing.T, src string, options naming.Options) []string {
	t.Helper()

	return reported(t, src, options, naming.MaxWordsRule)
}

func expect(t *testing.T, got []string, want ...string) {
	t.Helper()

	if len(got) == 0 && len(want) == 0 {
		return
	}

	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %q\nwant %q", got, want)
	}
}

func TestSplit(t *testing.T) {
	cases := map[string][]string{
		"QueueRouter":       {"Queue", "Router"},
		"correlationID":     {"correlation", "ID"},
		"HTTPServer":        {"HTTP", "Server"},
		"userIDs":           {"user", "IDs"},
		"parseURLs":         {"parse", "URLs"},
		"URLsByHost":        {"URLs", "By", "Host"},
		"MAX_RETRY_COUNT":   {"MAX", "RETRY", "COUNT"},
		"oauth2Client":      {"oauth2", "Client"},
		"HTTPSProxy":        {"HTTPS", "Proxy"},
		"JSONData":          {"JSON", "Data"},
		"_":                 nil,
		"x":                 {"x"},
		"readUTF8":          {"read", "UTF8"},
		"ServeHTTPRequests": {"Serve", "HTTP", "Requests"},
	}

	for name, want := range cases {
		if got := naming.Split(name); !reflect.DeepEqual(got, want) {
			t.Errorf("Split(%q) = %q, want %q", name, got, want)
		}
	}
}

func TestOffByDefault(t *testing.T) {
	findings, err := naming.Check("a.go", []byte("package a\n\nfunc ReadBookingProposalTTL() {}\n"), naming.Options{})

	if err != nil || findings != nil {
		t.Fatalf("findings = %v, %v", findings, err)
	}
}

func TestParseErrorIsAnError(t *testing.T) {
	if _, err := naming.Check("a.go", []byte("package a\nfunc {"), naming.Options{MaxWords: 2}); err == nil {
		t.Fatal("no error")
	}
}

// calibration declares every kind of name at one, two and three words.
const calibration = `package worker

type Spec struct {
	Name        string
	QueueName   string
	MaxAttempts int
	RetryDelayMillis int
}

type Runner interface {
	Run() error
	RunOnce() error
	RunWithTimeout() error
}

const Limit = 1
const MaxJobs = 2
const DefaultQueueDepth = 3

var registry = map[string]Spec{}

func Start() {}
func StartPool() {}
func StartPoolWorkers() {}

func (s *Spec) Validate() {}
func (s *Spec) ValidateName() {}
func (s *Spec) ValidateQueueName() {}

func run(spec Spec, queueName string, maxRetryCount int) {
	n := 1
	jobCount := 2
	pendingJobCount := 3
	_, _, _ = n, jobCount, pendingJobCount
}
`

// TestMaxOneEverywhere pins MaxWords 1 with no local limit: every name of
// two or more words is reported.
func TestMaxOneEverywhere(t *testing.T) {
	expect(t, words(t, calibration, naming.Options{MaxWords: 1}),
		"QueueName", "MaxAttempts", "RetryDelayMillis",
		"RunOnce", "RunWithTimeout",
		"MaxJobs", "DefaultQueueDepth",
		"StartPool", "StartPoolWorkers",
		"ValidateName", "ValidateQueueName",
		"queueName", "maxRetryCount",
		"jobCount", "pendingJobCount",
	)
}

// TestMaxTwoEverywhere pins MaxWords 2: only names of three or more words.
func TestMaxTwoEverywhere(t *testing.T) {
	expect(t, words(t, calibration, naming.Options{MaxWords: 2}),
		"RetryDelayMillis", "RunWithTimeout", "DefaultQueueDepth", "StartPoolWorkers", "ValidateQueueName",
		"maxRetryCount", "pendingJobCount",
	)
}

// TestOneForLocalsTwoForTheRest pins MaxLocalWords 1 with MaxWords 2.
func TestOneForLocalsTwoForTheRest(t *testing.T) {
	expect(t, words(t, calibration, naming.Options{MaxWords: 2, MaxLocalWords: 1}),
		"RetryDelayMillis", "RunWithTimeout", "DefaultQueueDepth", "StartPoolWorkers", "ValidateQueueName",
		"queueName", "maxRetryCount", "jobCount", "pendingJobCount",
	)
}

// TestIdiomsAreNotCounted: New and Must on a constructor, Is and Has on a
// predicate, the Err prefix of a sentinel and the Err suffix of an error
// local keep max 1 workable.
func TestIdiomsAreNotCounted(t *testing.T) {
	src := `package a

import "errors"

var ErrNotFound = errors.New("x")
var ErrClosed = errors.New("y")

func NewServer() {}
func MustParse() {}
func IsReady() bool { return true }
func HasRoutes() (ok bool) { return }

type Conn struct{ HasTLS bool }

func (c *Conn) IsOpen() bool { return true }

func close(f interface{ Close() error }) {
	closeErr := f.Close()
	isEmpty := len("") == 0
	var hasRows bool
	_, _, _ = closeErr, isEmpty, hasRows
}
`

	// ErrNotFound still has two words after Err.
	expect(t, words(t, src, naming.Options{MaxWords: 1}), "ErrNotFound")

	// The idiom only counts where it is idiom.
	other := `package a

var IsReady = 1
var NewServer = 2

func IsRunning() int { return 0 }
func (s *S) NewChild() {}
func f() { serverErrCount := 1; _ = serverErrCount }
`

	expect(t, words(t, other, naming.Options{MaxWords: 1}), "IsReady", "NewServer", "IsRunning", "NewChild", "serverErrCount")
}

func TestMessagesPointAtThePackageAndTheReceiver(t *testing.T) {
	src := `package release

const MaxArchiveBytes = 1

type Archive struct{ SizeLimitBytes int }

func PackDirectory() {}
func NewArchiveWriter() {}
func (a *Archive) MaxEntryBytes() int { return 0 }
func f(sourceDir string) {}
`

	findings, err := naming.Check("a.go", []byte(src), naming.Options{MaxWords: 1})

	if err != nil {
		t.Fatal(err)
	}

	want := []string{
		`"MaxArchiveBytes" has 3 words, more than 1; let the package name carry the context: drop what package release already says, or move the name to a package or type named for it, as archive.Pack instead of release.PackDirectory`,
		`"SizeLimitBytes" has 3 words, more than 1; let the struct carry the context: drop what Archive already says, or group related fields under a field of their own type`,
		`"PackDirectory" has 2 words, more than 1; let the package name carry the context: drop what package release already says, or move the name to a package or type named for it, as archive.Pack instead of release.PackDirectory`,
		`"NewArchiveWriter" has 2 words, more than 1 (not counting "New"); let the package name carry the context: drop what package release already says, or move the name to a package or type named for it, as archive.Pack instead of release.PackDirectory`,
		`"MaxEntryBytes" has 3 words, more than 1; let the receiver type carry the context: callers read (*Archive).MaxEntryBytes, so drop what *Archive already says, or give the extra words a type of their own, as (*Archive).Limit instead of MaxArchiveBytes`,
		`"sourceDir" has 2 words, more than 1 for a local or parameter; it lives in a short scope, where the function, its receiver and its package already say the rest`,
	}

	var got []string

	for _, f := range findings {
		got = append(got, f.Message)
	}

	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got\n%s\nwant\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}

	if findings[0].Line != 3 || findings[0].Column != 7 || findings[0].Rule != naming.MaxWordsRule {
		t.Fatalf("first finding at %d:%d under %s", findings[0].Line, findings[0].Column, findings[0].Rule)
	}
}

func TestDictatedNamesAreSkipped(t *testing.T) {
	src := `package main

import longImportAlias "strings"

type Point struct {
	longImportAlias.Builder
}

func init() {}
func main() {}
func TestPackDirectoryWorks(t *T) {}
func BenchmarkPackDirectory(b *B) {}
func FuzzPackDirectory(f *F) {}
func ExamplePackDirectory() {}
func TestifySuite() {}

func (p Point) MarshalJSON() ([]byte, error) { return nil, nil }
func (p *Point) UnmarshalJSON(data []byte) error { return nil }
func (p Point) ServeHTTP(w W, r R) {}
func (p Point) GoString() string { return "" }

type Codec interface {
	MarshalText() ([]byte, error)
}

func Map[TElementType any](fn func(inputValue int) int) {
	for _, _ = range []int{} {
	}
}
`

	expect(t, words(t, src, naming.Options{MaxWords: 1}), "TestifySuite")
}

func TestEveryKindOfLocalIsJudged(t *testing.T) {
	src := `package a

func f(firstArg int) (namedResult int) {
	var localVar int
	const localConst = 1
	type localType struct{ fieldName int }
	shortDecl, otherDecl := 1, 2
	for loopKey, loopValue := range []int{} {
		_, _ = loopKey, loopValue
	}
	switch typedValue := any(1).(type) {
	default:
		_ = typedValue
	}
	callback := func(closureArg int) {
		innerLocal := 1
		_ = innerLocal
	}
	shortDecl, thirdDecl := 3, 4
	_, _, _, _, _ = localVar, otherDecl, callback, thirdDecl, localType{}
	return
}

func (receiverName *T) m() {}
`

	expect(t, words(t, src, naming.Options{MaxWords: 1}),
		"firstArg", "namedResult", "localVar", "localConst", "localType", "fieldName",
		"shortDecl", "otherDecl", "loopKey", "loopValue", "typedValue", "closureArg", "innerLocal", "thirdDecl", "receiverName",
	)
}

func TestPackageStutter(t *testing.T) {
	stutter := naming.Options{PackageStutter: true}
	src := `package worker

type Worker struct{}
type WorkerSpec struct{ WorkerName string }
type Workers []Worker
type workerState int

const WorkerLimit = 1

var WORKER_POOL_SIZE = 2

func NewWorker() *Worker { return nil }
func WorkerFor() {}
func (w *Worker) WorkerID() {}
func f() { WorkerLocal := 1; _ = WorkerLocal }
`

	expect(t, reported(t, src, stutter, naming.StutterRule), "WorkerSpec", "WorkerLimit", "WORKER_POOL_SIZE", "WorkerFor")

	findings, _ := naming.Check("a.go", []byte(src), stutter)
	want := `"WorkerSpec" starts with its package name: callers read worker.WorkerSpec; name it Spec, read as worker.Spec`

	if findings[0].Message != want {
		t.Fatalf("message = %s", findings[0].Message)
	}

	if want := "POOL_SIZE"; !strings.Contains(findings[2].Message, "name it "+want+",") {
		t.Fatalf("message = %s", findings[2].Message)
	}

	// A package name of several words, without case.
	multi := "package httputil\n\nfunc HTTPUtilProxy() {}\nfunc HTTPProxy() {}\n"
	expect(t, reported(t, multi, stutter, naming.StutterRule), "HTTPUtilProxy")

	// Package main and external test packages are not judged.
	expect(t, reported(t, "package main\n\nfunc MainLoop() {}\n", stutter, naming.StutterRule))
	expect(t, reported(t, "package worker_test\n\nfunc WorkerHelper() {}\n", stutter, naming.StutterRule))
}

func TestBothChecksReportTheSameName(t *testing.T) {
	src := "package worker\n\ntype WorkerQueueSpec struct{}\n"
	findings, err := naming.Check("a.go", []byte(src), naming.Options{MaxWords: 2, PackageStutter: true})

	if err != nil || len(findings) != 2 ||
		findings[0].Rule != naming.MaxWordsRule || findings[1].Rule != naming.StutterRule {
		t.Fatalf("findings = %+v, %v", findings, err)
	}
}

package cli

type options struct {
	mode         Mode
	configPath   string
	reportRoot   string
	outputFormat string
	hostPath     HostPath
	positional   []string
	// jobs overrides config.Concurrency when not -1. -1 means "unset"
	// (no override); 0 means "use NumCPU"; positive values pin the worker count.
	jobs int
}

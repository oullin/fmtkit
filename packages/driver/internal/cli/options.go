package cli

type options struct {
	mode         Mode
	configPath   string
	reportRoot   string
	outputFormat string
	hostPath     HostPath
	positional   []string
	// jobs overrides config.Concurrency when > 0. -1 means "unset".
	jobs int
}

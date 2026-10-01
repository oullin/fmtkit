import { defineConfig } from 'vite-plus';

const workspacePackages = ['--filter formatter', '--filter vet', '--filter driver', '--filter sidecar', '--fail-if-no-match'].join(' ');

const goPackages = ['--filter formatter', '--filter vet', '--filter driver', '--fail-if-no-match'].join(' ');

export default defineConfig({
	fmt: {
		arrowParens: 'always',
		printWidth: 200,
		semi: true,
		singleQuote: true,
		tabWidth: 4,
		trailingComma: 'all',
		useTabs: true,
	},
	// Lint rules live in .oxlintrc.json (the single source of truth); the
	// sidecar lint scripts invoke oxlint directly and discover it there.
	run: {
		cache: {
			scripts: true,
			tasks: true,
		},
		tasks: {
			check: `vp run ${workspacePackages} check`,
			// fmtkit formats itself with the binary it ships.
			format: './scripts/task.sh format',
			gofmt: './scripts/task.sh gofmt',
			'install-cli': './scripts/task.sh with-env go -C packages/go install ./driver/cmd/fmtkit-go',
			release: './scripts/release/release.sh',
			'test-race':
				'CGO_ENABLED=1 ./scripts/task.sh with-env go -C packages/go/formatter test ./... -race -v && CGO_ENABLED=1 ./scripts/task.sh with-env go -C packages/go/vet test ./... -race -v && CGO_ENABLED=1 ./scripts/task.sh with-env go -C packages/go/driver test ./... -race -v',
			'test:binary': './scripts/test-binary-smoke.sh',
			'test:coverage': './scripts/task.sh coverage',
			vet: `vp run ${goPackages} vet`,
		},
	},
});

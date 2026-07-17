import { defineConfig } from 'vite-plus';

const workspacePackages = ['--filter formatter', '--filter vet', '--filter driver', '--filter sidecar', '--fail-if-no-match'].join(' ');

const goPackages = ['--filter formatter', '--filter vet', '--filter driver', '--fail-if-no-match'].join(' ');

export default defineConfig({
	fmt: {
		semi: true,
		singleQuote: true,
		trailingComma: 'all',
		printWidth: 200,
		tabWidth: 4,
		useTabs: true,
		arrowParens: 'always',
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
			format: './infra/task.sh format',
			gofmt: './infra/task.sh gofmt',
			'install-cli': './infra/task.sh with-env go -C packages/go install ./driver/cmd/fmtkit-go',
			release: './infra/release/release.sh',
			'test:binary': './infra/test-binary-smoke.sh',
			'test:coverage': './infra/task.sh coverage',
			'test-race':
				'CGO_ENABLED=1 ./infra/task.sh with-env go -C packages/go/formatter test ./... -race -v && CGO_ENABLED=1 ./infra/task.sh with-env go -C packages/go/vet test ./... -race -v && CGO_ENABLED=1 ./infra/task.sh with-env go -C packages/go/driver test ./... -race -v',
			vet: `vp run ${goPackages} vet`,
		},
	},
});

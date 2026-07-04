import { defineConfig } from 'vite-plus';

const workspacePackages = ['--filter formatter', '--filter vet', '--filter driver', '--filter devx', '--fail-if-no-match'].join(' ');

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
	// devx lint scripts invoke oxlint directly and discover it there.
	run: {
		cache: {
			scripts: true,
			tasks: true,
		},
		tasks: {
			check: `vp run ${workspacePackages} check`,
			'check:docker': './scripts/format-docker.sh check',
			'docker:clean': './scripts/docker-image.sh clean',
			'format:docker': './scripts/format-docker.sh format',
			'format:local': './scripts/format.sh',
			gofmt: './scripts/fmt-source.sh',
			'image:full': './scripts/docker-image.sh full',
			'image:go': './scripts/docker-image.sh go',
			'image:node-ts': './scripts/docker-image.sh node-ts',
			'install-cli': './scripts/with-storage-env.sh go -C packages/driver install ./cmd/fmt-go',
			release: './scripts/release.sh',
			'test:coverage': './scripts/test-coverage.sh',
			'test:entrypoints': './scripts/test-entrypoints.sh',
			'test-race':
				'CGO_ENABLED=1 ./scripts/with-storage-env.sh go -C packages/formatter test ./... -race -v && CGO_ENABLED=1 ./scripts/with-storage-env.sh go -C packages/vet test ./... -race -v && CGO_ENABLED=1 ./scripts/with-storage-env.sh go -C packages/driver test ./... -race -v',
			vet: `vp run ${goPackages} vet`,
		},
	},
});

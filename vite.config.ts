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
			'check:docker': './infra/scripts/tasks/format-docker.sh check',
			'docker:clean': './infra/scripts/tasks/docker-image.sh clean',
			'format:docker': './infra/scripts/tasks/format-docker.sh format',
			'format:local': './infra/scripts/tasks/format.sh',
			gofmt: './infra/scripts/tasks/fmt-source.sh',
			'image:full': './infra/scripts/tasks/docker-image.sh full',
			'image:go': './infra/scripts/tasks/docker-image.sh go',
			'image:node-ts': './infra/scripts/tasks/docker-image.sh node-ts',
			'install-cli': './infra/scripts/tasks/with-storage-env.sh go install ./packages/driver/cmd/fmtkit-go',
			release: './infra/scripts/release/release.sh',
			'release:contained': './infra/scripts/tasks/release-contained.sh',
			'runtime:contained': './infra/scripts/tasks/package-contained-runtime.sh',
			'test:coverage': './infra/scripts/tasks/test-coverage.sh',
			'test:entrypoints': './infra/scripts/tasks/test-entrypoints.sh',
			'test-race':
				'CGO_ENABLED=1 ./infra/scripts/tasks/with-storage-env.sh go -C packages/formatter test ./... -race -v && CGO_ENABLED=1 ./infra/scripts/tasks/with-storage-env.sh go -C packages/vet test ./... -race -v && CGO_ENABLED=1 ./infra/scripts/tasks/with-storage-env.sh go -C packages/driver test ./... -race -v',
			vet: `vp run ${goPackages} vet`,
		},
	},
});

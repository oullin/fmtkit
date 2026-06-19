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
	lint: {
		plugins: ['typescript'],
		categories: {
			correctness: 'error',
		},
		rules: {
			'no-unused-vars': ['error', { args: 'after-used', argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrors: 'none' }],
			'no-self-compare': 'error',
			'no-template-curly-in-string': 'error',
			eqeqeq: ['error', 'always'],
			'no-var': 'error',
			'typescript/no-misused-new': 'error',
			'typescript/no-extra-non-null-assertion': 'error',
			'typescript/no-non-null-asserted-optional-chain': 'error',
			'typescript/no-duplicate-enum-values': 'error',
			'typescript/no-unsafe-declaration-merging': 'error',
			'typescript/prefer-as-const': 'error',
			'typescript/consistent-type-imports': 'error',
		},
	},
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

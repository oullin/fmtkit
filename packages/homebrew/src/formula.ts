import { constants as fileConstants, lstat, mkdir, open, readdir, realpath, rename, rm } from 'node:fs/promises';
import type { FileHandle } from 'node:fs/promises';
import path from 'node:path';

export const RELEASE_ASSETS = Object.freeze(['fmtkit-darwin-arm64', 'fmtkit-linux-amd64', 'fmtkit-linux-arm64'] as const);

export type ReleaseAsset = (typeof RELEASE_ASSETS)[number];

export interface FormulaOptions {
	tag: string;
	checksumsDir: string;
	output: string;
}

export type Checksums = Readonly<Record<ReleaseAsset, string>>;

interface FormulaGeneration {
	formula: string;
	output: string;
	protectedInputs: readonly string[];
}

const TAG_PATTERN = /^v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)$/;
const CHECKSUM_SUFFIX = '.sha256';

let temporaryFileSequence = 0;

export async function generateFormulaFromArguments(arguments_: unknown): Promise<void> {
	const options = parseArguments(arguments_);

	const generation = await prepareFormulaGeneration(options);

	await writeFileAtomically(generation.output, generation.formula, generation.protectedInputs);
}

export function parseArguments(arguments_: unknown): FormulaOptions {
	if (!Array.isArray(arguments_)) {
		throw new TypeError('arguments must be an array');
	}

	const values = new Map<string, string>();
	const allowed = new Set(['--tag', '--checksums-dir', '--output']);

	for (let index = 0; index < arguments_.length; index += 2) {
		const flag = arguments_[index];
		const value = arguments_[index + 1];

		if (typeof flag !== 'string' || !allowed.has(flag)) {
			throw new Error(`unknown argument: ${String(flag)}`);
		}

		if (values.has(flag)) {
			throw new Error(`duplicate argument: ${flag}`);
		}

		if (typeof value !== 'string' || value.length === 0 || allowed.has(value)) {
			throw new Error(`missing value for ${flag}`);
		}

		values.set(flag, value);
	}

	for (const flag of allowed) {
		if (!values.has(flag)) {
			throw new Error(`missing required argument: ${flag}`);
		}
	}

	return {
		tag: validateTag(values.get('--tag')),
		checksumsDir: path.resolve(values.get('--checksums-dir')!),
		output: path.resolve(values.get('--output')!),
	};
}

export async function generateFormula({ tag, checksumsDir, output }: FormulaOptions): Promise<string> {
	const generation = await prepareFormulaGeneration({ tag, checksumsDir, output });

	return generation.formula;
}

async function prepareFormulaGeneration({ tag, checksumsDir, output }: FormulaOptions): Promise<FormulaGeneration> {
	validateTag(tag);

	if (typeof checksumsDir !== 'string' || checksumsDir.length === 0) {
		throw new Error('checksums directory must be a non-empty path');
	}

	if (typeof output !== 'string' || output.length === 0) {
		throw new Error('output must be a non-empty path');
	}

	const resolvedChecksumsDir = path.resolve(checksumsDir);

	const checksums = await readChecksums(resolvedChecksumsDir);

	const canonicalChecksumsDir = await realpath(resolvedChecksumsDir);

	const canonicalOutput = await resolveCanonicalOutput(output);

	const inputPaths = RELEASE_ASSETS.map((asset) => path.join(canonicalChecksumsDir, `${asset}${CHECKSUM_SUFFIX}`));

	if (inputPaths.includes(canonicalOutput)) {
		throw new Error('output must not overwrite a checksum input');
	}

	return {
		formula: renderFormula(tag, checksums),
		output: canonicalOutput,
		protectedInputs: inputPaths,
	};
}

export function validateTag(tag: unknown): string {
	if (typeof tag !== 'string' || !TAG_PATTERN.test(tag)) {
		throw new Error(`invalid release tag: ${String(tag)} (expected vMAJOR.MINOR.PATCH)`);
	}

	return tag;
}

export async function readChecksums(checksumsDir: string): Promise<Checksums> {
	const directory = path.resolve(checksumsDir);

	const directoryMetadata = await safeLstat(directory, 'checksums directory');

	if (directoryMetadata.isSymbolicLink() || !directoryMetadata.isDirectory()) {
		throw new Error(`checksums directory is not a real directory: ${directory}`);
	}

	const entries = await readdir(directory, { withFileTypes: true });

	const expectedChecksumFiles = new Set(RELEASE_ASSETS.map((asset) => `${asset}${CHECKSUM_SUFFIX}`));

	const unexpectedChecksumFiles = entries
		.map((entry) => entry.name)
		.filter((name) => name.endsWith(CHECKSUM_SUFFIX) && !expectedChecksumFiles.has(name))
		.sort();

	if (unexpectedChecksumFiles.length > 0) {
		throw new Error(`unexpected checksum file: ${unexpectedChecksumFiles.join(', ')}`);
	}

	const checksums: Partial<Record<ReleaseAsset, string>> = {};
	const identities = new Set<string>();

	for (const asset of RELEASE_ASSETS) {
		const filename = `${asset}${CHECKSUM_SUFFIX}`;
		const checksumPath = path.join(directory, filename);

		const metadata = await safeLstat(checksumPath, `checksum file ${filename}`);

		if (metadata.isSymbolicLink() || !metadata.isFile()) {
			throw new Error(`checksum input is not a regular file: ${filename}`);
		}

		const identity = `${metadata.dev}:${metadata.ino}`;

		if (identities.has(identity)) {
			throw new Error(`duplicate checksum input: ${filename}`);
		}

		identities.add(identity);

		const contents = await readChecksumWithoutFollowingLinks(checksumPath, filename);

		const match = contents.match(new RegExp(`^([a-f0-9]{64})  ${asset}\\n$`));

		if (match === null) {
			throw new Error(`malformed checksum file ${filename}: expected 64 lowercase hex characters, two spaces, the exact asset filename, and a trailing newline`);
		}

		checksums[asset] = match[1];
	}

	return Object.freeze(checksums) as Checksums;
}

export function renderFormula(tag: string, checksums: Checksums): string {
	validateTag(tag);

	for (const asset of RELEASE_ASSETS) {
		if (typeof checksums?.[asset] !== 'string' || !/^[a-f0-9]{64}$/.test(checksums[asset])) {
			throw new Error(`invalid checksum for ${asset}`);
		}
	}

	const version = tag.slice(1);
	const releaseRoot = `https://github.com/oullin/fmtkit/releases/download/${tag}`;

	return `class Fmtkit < Formula
  desc "Contained Go, TypeScript, and Vue formatter"
  homepage "https://github.com/oullin/fmtkit"
  version "${version}"

  on_macos do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "${releaseRoot}/fmtkit-darwin-arm64", using: :nounzip
      sha256 "${checksums['fmtkit-darwin-arm64']}"
    else
      odie "fmtkit does not support Intel macOS; Apple Silicon is required"
    end
  end

  on_linux do
    if Hardware::CPU.intel? && Hardware::CPU.is_64_bit?
      url "${releaseRoot}/fmtkit-linux-amd64", using: :nounzip
      sha256 "${checksums['fmtkit-linux-amd64']}"
    elsif Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "${releaseRoot}/fmtkit-linux-arm64", using: :nounzip
      sha256 "${checksums['fmtkit-linux-arm64']}"
    else
      odie "fmtkit supports only 64-bit x86 and ARM Linux"
    end
  end

  def install
    asset = if OS.mac?
      "fmtkit-darwin-arm64"
    elsif Hardware::CPU.arm?
      "fmtkit-linux-arm64"
    else
      "fmtkit-linux-amd64"
    end

    bin.install asset => "fmtkit"
  end

  test do
    full_fixture = testpath/"full-non-git-fixture"
    full_fixture.mkpath
    (full_fixture/"go.mod").write "module example.com/fullfixture\\n\\ngo 1.26.4\\n"
    (full_fixture/"main.go").write "package fullfixture\\n\\nfunc answer( )int{return 42}\\n"
    (full_fixture/"input.ts").write "export const answer={value:42}\\n"
    (full_fixture/"Component.vue").write <<~EOS
      <script setup lang="ts">
      const message="ok"
      </script>
      <template><main>{{ message }}</main></template>
    EOS
    full_fixture.cd { system bin/"fmtkit" }

    go_fixture = testpath/"go-mode-fixture"
    go_fixture.mkpath
    (go_fixture/"go.mod").write "module example.com/gofixture\\n\\ngo 1.26.4\\n"
    (go_fixture/"main.go").write "package gofixture\\n\\nfunc answer( )int{return 42}\\n"
    go_fixture.cd { system bin/"fmtkit", "--go", "." }

    ts_fixture = testpath/"ts-mode-fixture"
    ts_fixture.mkpath
    (ts_fixture/"input.ts").write "export const answer={value:42}\\n"
    (ts_fixture/"Component.vue").write <<~EOS
      <script setup lang="ts">
      const message="ok"
      </script>
      <template><main>{{ message }}</main></template>
    EOS
    ts_fixture.cd { system bin/"fmtkit", "--ts", "input.ts", "Component.vue" }
  end
end
`;
}

async function safeLstat(filePath: string, label: string) {
	try {
		return await lstat(filePath);
	} catch (error) {
		if (isNodeError(error) && error.code === 'ENOENT') {
			throw new Error(`missing ${label}: ${filePath}`);
		}

		throw error;
	}
}

async function readChecksumWithoutFollowingLinks(checksumPath: string, filename: string): Promise<string> {
	let handle: FileHandle | undefined;

	try {
		handle = await open(checksumPath, fileConstants.O_RDONLY | (fileConstants.O_NOFOLLOW ?? 0));

		return await handle.readFile({ encoding: 'utf8' });
	} catch (error) {
		if (isNodeError(error) && error.code === 'ELOOP') {
			throw new Error(`checksum input must not be a symbolic link: ${filename}`);
		}

		throw error;
	} finally {
		await handle?.close();
	}
}

async function resolveCanonicalOutput(output: string): Promise<string> {
	const target = path.resolve(output);
	const targetDirectory = path.dirname(target);

	await mkdir(targetDirectory, { recursive: true });

	return path.join(await realpath(targetDirectory), path.basename(target));
}

async function writeFileAtomically(output: string, contents: string, protectedInputs: readonly string[]): Promise<void> {
	const target = path.resolve(output);
	const targetDirectory = path.dirname(target);

	await mkdir(targetDirectory, { recursive: true });

	try {
		const targetMetadata = await lstat(target);

		if (targetMetadata.isSymbolicLink() || !targetMetadata.isFile()) {
			throw new Error(`output is not a regular file: ${target}`);
		}
	} catch (error) {
		if (!isNodeError(error) || error.code !== 'ENOENT') {
			throw error;
		}
	}

	const canonicalDirectory = await realpath(targetDirectory);

	const canonicalTarget = path.join(canonicalDirectory, path.basename(target));

	if (protectedInputs.includes(canonicalTarget)) {
		throw new Error('output must not overwrite a checksum input');
	}

	const temporaryPath = path.join(canonicalDirectory, `.${path.basename(canonicalTarget)}.${process.pid}.${(temporaryFileSequence += 1)}.tmp`);

	let handle: FileHandle | undefined;

	try {
		handle = await open(temporaryPath, 'wx', 0o600);

		await handle.writeFile(contents, { encoding: 'utf8' });

		await handle.chmod(0o644);

		await handle.sync();

		await handle.close();

		handle = undefined;

		await rename(temporaryPath, canonicalTarget);
	} catch (error) {
		await handle?.close();

		await rm(temporaryPath, { force: true });

		throw error;
	}
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
	return error instanceof Error && 'code' in error;
}

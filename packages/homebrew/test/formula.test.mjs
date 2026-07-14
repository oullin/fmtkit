import assert from 'node:assert/strict';
import { execFile, spawnSync } from 'node:child_process';
import { link, mkdtemp, mkdir, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import test from 'node:test';

import { RELEASE_ASSETS, generateFormula, generateFormulaFromArguments, parseArguments, readChecksums, renderFormula, validateTag } from '../src/formula.mjs';

const execFileAsync = promisify(execFile);
const GENERATOR_SCRIPT = fileURLToPath(new URL('../scripts/generate-formula.mjs', import.meta.url));
const DIGESTS = Object.freeze({
	'fmtkit-darwin-arm64': '1'.repeat(64),
	'fmtkit-linux-amd64': '2'.repeat(64),
	'fmtkit-linux-arm64': '3'.repeat(64),
});

test('generates a deterministic formula with every supported platform', async () => {
	const directory = await makeChecksumDirectory();
	const firstOutput = path.join(directory, 'Formula', 'fmtkit.rb');
	const secondOutput = path.join(directory, 'other', 'fmtkit.rb');
	const options = { tag: 'v1.2.3', checksumsDir: directory };

	await generateFormulaFromArguments(['--tag', options.tag, '--checksums-dir', directory, '--output', firstOutput]);
	await generateFormulaFromArguments(['--output', secondOutput, '--checksums-dir', directory, '--tag', options.tag]);

	const first = await readFile(firstOutput, 'utf8');
	const second = await readFile(secondOutput, 'utf8');
	assert.equal(first, second);
	assert.equal(first, await generateFormula({ ...options, output: firstOutput }));
	assert.match(first, /^class Fmtkit < Formula$/m);
	assert.match(first, /^  version "1\.2\.3"$/m);

	for (const asset of RELEASE_ASSETS) {
		assert.match(first, new RegExp(`url "https://github\\.com/oullin/fmtkit/releases/download/v1\\.2\\.3/${asset}", using: :nounzip`));
		assert.match(first, new RegExp(`sha256 "${DIGESTS[asset]}"`));
	}

	assert.match(first, /Hardware::CPU\.intel\? && Hardware::CPU\.is_64_bit\?/);
	assert.match(first, /Hardware::CPU\.arm\? && Hardware::CPU\.is_64_bit\?/);
	assert.match(first, /fmtkit does not support Intel macOS; Apple Silicon is required/);
	assert.match(first, /fmtkit supports only 64-bit x86 and ARM Linux/);
});

test('installs exactly one executable named fmtkit and exercises contained modes', () => {
	const formula = renderFormula('v9.8.7', DIGESTS);
	const installBlock = formula.match(/  def install\n(?<body>[\s\S]*?)\n  end\n\n  test do/)?.groups?.body;

	assert.ok(installBlock);
	assert.match(installBlock, /bin\.install asset => "fmtkit"/);
	assert.equal(installBlock.match(/bin\.install/g)?.length, 1);
	assert.doesNotMatch(formula, /fmt-all|symlink|alias/i);
	assert.match(formula, /full_fixture\.cd \{ system bin\/"fmtkit" \}/);
	assert.match(formula, /go_fixture\.cd \{ system bin\/"fmtkit", "--go", "\." \}/);
	assert.match(formula, /ts_fixture\.cd \{ system bin\/"fmtkit", "--ts", "input\.ts", "Component\.vue" \}/);
	assert.match(formula, /full-non-git-fixture/);
	assert.doesNotMatch(formula, /system (?:"|bin\/)(?:node|go|docker)(?:"|\/)/);
});

test('validates release tags strictly and resists formula injection', () => {
	for (const tag of ['1.2.3', 'v1.2', 'v1.2.3.4', 'v01.2.3', 'v1.02.3', 'v1.2.03', 'v1.2.3-rc.1', 'v1.2.3\nend', 'v1.2.3"; system "id"', ' v1.2.3']) {
		assert.throws(() => validateTag(tag), /invalid release tag/);
	}

	assert.equal(validateTag('v0.0.0'), 'v0.0.0');
	assert.equal(validateTag('v123.456.789'), 'v123.456.789');
	assert.throws(() => renderFormula('v1.2.3', { ...DIGESTS, 'fmtkit-linux-arm64': '3'.repeat(63) + '"' }), /invalid checksum/);
});

test('parses exactly one value for every CLI argument', () => {
	const parsed = parseArguments(['--tag', 'v1.2.3', '--checksums-dir', 'checksums', '--output', 'Formula/fmtkit.rb']);

	assert.equal(parsed.tag, 'v1.2.3');
	assert.equal(parsed.checksumsDir, path.resolve('checksums'));
	assert.equal(parsed.output, path.resolve('Formula/fmtkit.rb'));
	assert.throws(() => parseArguments(['--tag', 'v1.2.3']), /missing required argument/);
	assert.throws(() => parseArguments(['--tag', 'v1.2.3', '--tag', 'v2.0.0', '--checksums-dir', 'x', '--output', 'y']), /duplicate argument/);
	assert.throws(() => parseArguments(['--wat', 'value']), /unknown argument/);
	assert.throws(() => parseArguments(['--tag', '--output']), /missing value/);
});

test('strictly parses checksum filenames and contents', async (t) => {
	const malformedCases = [
		['uppercase digest', 'A'.repeat(64) + '  fmtkit-linux-amd64\n'],
		['short digest', 'a'.repeat(63) + '  fmtkit-linux-amd64\n'],
		['single separator space', 'a'.repeat(64) + ' fmtkit-linux-amd64\n'],
		['wrong asset', 'a'.repeat(64) + '  fmtkit-linux-arm64\n'],
		['missing trailing newline', 'a'.repeat(64) + '  fmtkit-linux-amd64'],
		['CRLF ending', 'a'.repeat(64) + '  fmtkit-linux-amd64\r\n'],
		['duplicate line', `${'a'.repeat(64)}  fmtkit-linux-amd64\n${'a'.repeat(64)}  fmtkit-linux-amd64\n`],
		['shell payload', `${'a'.repeat(64)}  fmtkit-linux-amd64; touch owned\n`],
	];

	for (const [name, contents] of malformedCases) {
		await t.test(name, async () => {
			const directory = await makeChecksumDirectory();
			await writeFile(path.join(directory, 'fmtkit-linux-amd64.sha256'), contents);
			await assert.rejects(readChecksums(directory), /malformed checksum file/);
		});
	}
});

test('rejects missing, unexpected, duplicate, and unsafe checksum inputs', async (t) => {
	await t.test('missing platform', async () => {
		const directory = await makeChecksumDirectory();
		await rm(path.join(directory, 'fmtkit-linux-arm64.sha256'));
		await assert.rejects(readChecksums(directory), /missing checksum file fmtkit-linux-arm64\.sha256/);
	});

	await t.test('unexpected checksum filename', async () => {
		const directory = await makeChecksumDirectory();
		await writeFile(path.join(directory, 'fmtkit-windows-amd64.sha256'), `${'4'.repeat(64)}  fmtkit-windows-amd64\n`);
		await assert.rejects(readChecksums(directory), /unexpected checksum file: fmtkit-windows-amd64\.sha256/);
	});

	await t.test('hard-linked duplicate inputs', async () => {
		const directory = await makeChecksumDirectory();
		const duplicate = path.join(directory, 'fmtkit-linux-arm64.sha256');
		await rm(duplicate);
		await link(path.join(directory, 'fmtkit-linux-amd64.sha256'), duplicate);
		await assert.rejects(readChecksums(directory), /duplicate checksum input/);
	});

	await t.test('symbolic link input', { skip: process.platform === 'win32' }, async () => {
		const directory = await makeChecksumDirectory();
		const checksumPath = path.join(directory, 'fmtkit-linux-arm64.sha256');
		await rm(checksumPath);
		await symlink(path.join(directory, 'fmtkit-linux-amd64.sha256'), checksumPath);
		await assert.rejects(readChecksums(directory), /not a regular file/);
	});

	await t.test('symbolic link directory', { skip: process.platform === 'win32' }, async () => {
		const root = await mkdtemp(path.join(os.tmpdir(), 'fmtkit-homebrew-link-'));
		const directory = await makeChecksumDirectory();
		const linkPath = path.join(root, 'checksums');
		await symlink(directory, linkPath);
		await assert.rejects(readChecksums(linkPath), /not a real directory/);
	});
});

test('writes atomically and never changes an existing output after validation failure', async () => {
	const directory = await makeChecksumDirectory();
	const outputDirectory = path.join(directory, 'Formula');
	const output = path.join(outputDirectory, 'fmtkit.rb');
	await mkdir(outputDirectory);
	await writeFile(output, 'previous formula\n');
	await writeFile(path.join(directory, 'fmtkit-linux-arm64.sha256'), 'invalid\n');

	await assert.rejects(generateFormulaFromArguments(['--tag', 'v1.2.3', '--checksums-dir', directory, '--output', output]), /malformed checksum file/);
	assert.equal(await readFile(output, 'utf8'), 'previous formula\n');
	assert.deepEqual((await readdir(outputDirectory)).sort(), ['fmtkit.rb']);
});

test('refuses to overwrite an input checksum', async () => {
	const directory = await makeChecksumDirectory();
	const output = path.join(directory, 'fmtkit-linux-arm64.sha256');
	await assert.rejects(generateFormula({ tag: 'v1.2.3', checksumsDir: directory, output }), /must not overwrite/);
});

test('refuses to overwrite an input checksum through a symlinked output parent', { skip: process.platform === 'win32' }, async () => {
	const root = await mkdtemp(path.join(os.tmpdir(), 'fmtkit-homebrew-output-link-'));
	const checksumsDirectory = await makeChecksumDirectory();
	const checksumPath = path.join(checksumsDirectory, 'fmtkit-linux-arm64.sha256');
	const originalChecksum = await readFile(checksumPath, 'utf8');
	const outputAlias = path.join(root, 'checksums-alias');
	await symlink(checksumsDirectory, outputAlias);

	await assert.rejects(
		generateFormulaFromArguments(['--tag', 'v1.2.3', '--checksums-dir', checksumsDirectory, '--output', path.join(outputAlias, 'fmtkit-linux-arm64.sha256')]),
		/must not overwrite/,
	);
	assert.equal(await readFile(checksumPath, 'utf8'), originalChecksum);
});

test('generated formula passes Ruby syntax verification when Ruby is available', async (t) => {
	if (spawnSync('ruby', ['--version'], { encoding: 'utf8' }).error?.code === 'ENOENT') {
		t.skip('Ruby is unavailable');
		return;
	}

	const directory = await makeChecksumDirectory();
	const output = path.join(directory, 'Formula', 'fmtkit.rb');
	await generateFormulaFromArguments(['--tag', 'v1.2.3', '--checksums-dir', directory, '--output', output]);
	const { stdout, stderr } = await execFileAsync('ruby', ['-c', output]);
	assert.match(stdout, /Syntax OK/);
	assert.equal(stderr, '');
});

test('CLI exits non-zero without writing for invalid input', async () => {
	const directory = await makeChecksumDirectory();
	const output = path.join(directory, 'Formula', 'fmtkit.rb');

	await assert.rejects(execFileAsync(process.execPath, [GENERATOR_SCRIPT, '--tag', 'v1.2.3\nend', '--checksums-dir', directory, '--output', output]), (error) => {
		assert.equal(error.code, 1);
		assert.match(error.stderr, /invalid release tag/);
		return true;
	});
});

test('CLI accepts the package-manager separator and writes Formula/fmtkit.rb', async () => {
	const directory = await makeChecksumDirectory();
	const output = path.join(directory, 'Formula', 'fmtkit.rb');
	const { stdout, stderr } = await execFileAsync(process.execPath, [GENERATOR_SCRIPT, '--', '--tag', 'v4.5.6', '--checksums-dir', directory, '--output', output]);

	assert.equal(stdout, '');
	assert.equal(stderr, '');
	assert.match(await readFile(output, 'utf8'), /releases\/download\/v4\.5\.6\/fmtkit-darwin-arm64/);
});

async function makeChecksumDirectory() {
	const directory = await mkdtemp(path.join(os.tmpdir(), 'fmtkit-homebrew-'));

	for (const asset of RELEASE_ASSETS) {
		await writeFile(path.join(directory, `${asset}.sha256`), `${DIGESTS[asset]}  ${asset}\n`, {
			mode: 0o600,
		});
	}

	await writeFile(path.join(directory, 'release-notes.txt'), 'unrelated release artifact\n');

	return directory;
}

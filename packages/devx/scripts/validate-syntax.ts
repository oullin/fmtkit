import { readFile } from 'node:fs/promises';
import { parseSync } from 'oxc-parser';
import { collectSourceFiles } from '#devx/source-files';

const cwd = process.cwd();
const VUE_SCRIPT_REGEX = /(<script\b[^>]*>)([\s\S]*?)(<\/script>)/g;

type ParseError = {
	message?: unknown;
	codeframe?: unknown;
};

type ParseResult = {
	errors?: ParseError[];
};

function parseErrors(virtualName: string, content: string): ParseError[] {
	const parsed = parseSync(virtualName, content) as ParseResult;

	return parsed.errors ?? [];
}

function formatError(file: string, error: ParseError): string {
	if (typeof error.codeframe === 'string' && error.codeframe.length > 0) {
		return `[validate-syntax] ${file}\n${error.codeframe.trimEnd()}`;
	}

	if (typeof error.message === 'string' && error.message.length > 0) {
		return `[validate-syntax] ${file}: ${error.message}`;
	}

	return `[validate-syntax] ${file}: syntax validation failed`;
}

function validateVue(file: string, content: string): string[] {
	const failures: string[] = [];

	VUE_SCRIPT_REGEX.lastIndex = 0;

	let match: RegExpExecArray | null;

	while ((match = VUE_SCRIPT_REGEX.exec(content)) !== null) {
		const script = match[2];
		const errors = parseErrors(`${file}.script.ts`, script);

		for (const error of errors) {
			failures.push(formatError(file, error));
		}
	}

	return failures;
}

async function validateFile(file: string): Promise<string[]> {
	const content = await readFile(file, 'utf8').catch((err: unknown) => {
		if (typeof err === 'object' && err !== null && 'code' in err && err.code === 'ENOENT') {
			console.warn(`[validate-syntax] path not found, skipping: ${file}`);

			return null;
		}

		throw err;
	});

	if (content === null) {
		return [];
	}

	if (file.endsWith('.vue')) {
		return validateVue(file, content);
	}

	return parseErrors(file, content).map((error) => {
		return formatError(file, error);
	});
}

async function main(): Promise<void> {
	const files = await collectSourceFiles(process.argv.slice(2), true, 'validate-syntax');

	const failures: string[] = [];

	for (const file of files) {
		failures.push(...(await validateFile(file)));
	}

	if (failures.length > 0) {
		console.error(failures.join('\n'));
		console.error(`[validate-syntax] ${failures.length} syntax error(s) found after formatting.`);
		process.exit(1);
	}

	console.log(`[validate-syntax] checked ${files.length} file(s) in ${cwd}`);
}

main().catch((err: unknown) => {
	console.error(err);
	process.exit(1);
});

import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { parseSync } from 'oxc-parser';
import { extractVueScripts, isJavaScriptOrTypeScript, isNotFoundError, scriptAttribute } from '#sidecar/pass-utils';

const cwd = process.cwd();

type ParseError = {
	message?: unknown;
	codeframe?: unknown;
};

function parseErrors(virtualName: string, content: string): ParseError[] {
	const parsed = parseSync(virtualName, content);

	return parsed.errors;
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

function scriptExtension(openingTag: string): 'ts' | 'tsx' {
	const lang = scriptAttribute(openingTag, 'lang') ?? '';

	return lang === 'tsx' || lang === 'jsx' ? 'tsx' : 'ts';
}

function scriptPrefix(content: string, scriptStart: number): string {
	return content.slice(0, scriptStart).replace(/[^\r\n]/g, ' ');
}

function validateVue(file: string, content: string): string[] {
	const failures: string[] = [];

	for (const block of extractVueScripts(content)) {
		if (!isJavaScriptOrTypeScript(block.openTag)) {
			continue;
		}

		const virtualContent = scriptPrefix(content, block.start) + block.content;
		const errors = parseErrors(`${file}.script.${scriptExtension(block.openTag)}`, virtualContent);

		for (const error of errors) {
			failures.push(formatError(file, error));
		}
	}

	return failures;
}

export async function validateFile(file: string): Promise<string[]> {
	const content = await readFile(file, 'utf8').catch((err: unknown) => {
		if (isNotFoundError(err)) {
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
	const files = process.argv.slice(2).filter((file) => {
		return file.endsWith('.ts') || file.endsWith('.vue');
	});

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

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	main().catch((err: unknown) => {
		console.error(err);
		process.exit(1);
	});
}

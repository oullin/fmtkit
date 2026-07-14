#!/usr/bin/env node

import { generateFormulaFromArguments } from '../src/formula.ts';

try {
	const arguments_ = process.argv.slice(2);

	if (arguments_[0] === '--') {
		arguments_.shift();
	}

	await generateFormulaFromArguments(arguments_);
} catch (error) {
	const message = error instanceof Error ? error.message : String(error);

	process.stderr.write(`homebrew formula generation failed: ${message}\n`);
	process.exitCode = 1;
}

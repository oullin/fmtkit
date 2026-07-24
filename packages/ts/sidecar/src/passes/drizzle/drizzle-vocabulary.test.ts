import assert from 'node:assert/strict';
import { test } from 'node:test';
import { DrizzleVocabulary } from '#sidecar/passes/drizzle/drizzle-vocabulary';

test('DrizzleVocabulary.standard classifies recognised Drizzle names', () => {
	const vocabulary = DrizzleVocabulary.standard();

	assert.equal(vocabulary.isConventionalReceiver('db'), true);
	assert.equal(vocabulary.isConventionalReceiver('tx'), true);
	assert.equal(vocabulary.isConventionalReceiver('builder'), false);

	assert.equal(vocabulary.isChainMethod('select'), true);
	assert.equal(vocabulary.isChainMethod('from'), true);
	assert.equal(vocabulary.isChainMethod('findMany'), false);

	assert.equal(vocabulary.isFormatMethod('where'), true);
	assert.equal(vocabulary.isFormatMethod('findMany'), true);
	assert.equal(vocabulary.isFormatMethod('from'), false);

	assert.equal(vocabulary.isHelper('eq'), true);
	assert.equal(vocabulary.isHelper('and'), true);
	assert.equal(vocabulary.isHelper('coalesce'), false);

	assert.equal(vocabulary.isMultilineHelper('and'), true);
	assert.equal(vocabulary.isMultilineHelper('exists'), true);
	assert.equal(vocabulary.isMultilineHelper('eq'), false);

	assert.equal(vocabulary.isSetOperation('union'), true);
	assert.equal(vocabulary.isSetOperation('unionAll'), true);
	assert.equal(vocabulary.isSetOperation('where'), false);

	assert.equal(vocabulary.formatsObjectKey('with'), true);
	assert.equal(vocabulary.formatsObjectKey('target'), true);
	assert.equal(vocabulary.formatsObjectKey('id'), false);
});

test('DrizzleVocabulary is frozen and self-contained per instance', () => {
	const vocabulary = DrizzleVocabulary.standard();

	assert.equal(Object.isFrozen(vocabulary), true);

	const custom = new DrizzleVocabulary({
		receivers: ['store'],
		chainMethods: ['select'],
		formatMethods: ['where'],
		helpers: ['eq'],
		multilineHelpers: ['and'],
		setOperations: ['union'],
		objectKeys: ['with'],
	});

	assert.equal(custom.isConventionalReceiver('store'), true);
	assert.equal(custom.isConventionalReceiver('db'), false);
	assert.equal(DrizzleVocabulary.standard().isConventionalReceiver('store'), false);
});

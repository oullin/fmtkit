import { parseSync } from 'oxc-parser';
import { collectStatementLists, getEnd, getStart, visit } from '#devx/ast';
import type { Edit, Node } from '#devx/types';

function isMultiline(source: string, node: Node): boolean {
	const start = getStart(node);
	const end = getEnd(node);

	return start >= 0 && end >= 0 && source.slice(start, end).includes('\n');
}

function isConstDeclaration(node: Node): boolean {
	return node.type === 'VariableDeclaration' && (node as { kind?: unknown }).kind === 'const';
}

function lineStart(source: string, pos: number): number {
	return source.lastIndexOf('\n', pos - 1) + 1;
}

function lineIndent(source: string, pos: number): string {
	const start = lineStart(source, pos);
	const match = source.slice(start, pos).match(/^[ \t]*/);

	return match?.[0] ?? '';
}

function nodeSource(source: string, node: Node): string {
	const start = getStart(node);
	const end = getEnd(node);

	return `${lineIndent(source, start)}${source.slice(start, end)}`;
}

function isSideEffectSafeExpression(node: Node | undefined): boolean {
	if (!node) {
		return true;
	}

	return ['ArrayExpression', 'ArrowFunctionExpression', 'FunctionExpression', 'Literal', 'ObjectExpression', 'TemplateLiteral'].includes(node.type);
}

function isSafeConstDeclaration(node: Node): boolean {
	if (!isConstDeclaration(node)) {
		return false;
	}

	const declarations = node.declarations as Node[] | undefined;

	return (
		Array.isArray(declarations) &&
		declarations.every((declaration) => {
			return isSideEffectSafeExpression(declaration.init as Node | undefined);
		})
	);
}

function declaredNames(nodes: Node[]): Set<string> {
	const names = new Set<string>();

	for (const node of nodes) {
		const declarations = node.declarations as Node[] | undefined;

		if (!Array.isArray(declarations)) {
			continue;
		}

		for (const declaration of declarations) {
			const id = declaration.id as Node | undefined;
			const name = (id as { name?: unknown } | undefined)?.name;

			if (id?.type === 'Identifier' && typeof name === 'string') {
				names.add(name);
			}
		}
	}

	return names;
}

function usesAnyIdentifier(node: Node, names: Set<string>): boolean {
	let found = false;

	visit(node, (child) => {
		if (found || child.type !== 'Identifier') {
			return;
		}

		const name = (child as { name?: unknown }).name;

		if (typeof name === 'string' && names.has(name)) {
			found = true;
		}
	});

	return found;
}

function canReorderConstGroup(source: string, group: Node[]): boolean {
	if (
		!group.every((node) => {
			return isSafeConstDeclaration(node);
		})
	) {
		return false;
	}

	for (let i = 0; i < group.length; i++) {
		if (!isMultiline(source, group[i])) {
			continue;
		}

		const names = declaredNames([group[i]]);

		if (
			group.slice(i + 1).some((node) => {
				return !isMultiline(source, node) && usesAnyIdentifier(node, names);
			})
		) {
			return false;
		}
	}

	return true;
}

function splitGroups(list: Node[], predicate: (node: Node) => boolean): Node[][] {
	const groups: Node[][] = [];

	let current: Node[] = [];

	for (const node of list) {
		if (!predicate(node)) {
			if (current.length > 1) {
				groups.push(current);
			}

			current = [];
			continue;
		}

		current.push(node);
	}

	if (current.length > 1) {
		groups.push(current);
	}

	return groups;
}

function groupEdit(source: string, group: Node[], canReorder: boolean): Edit | null {
	const singleLine = group.filter((node) => {
		return !isMultiline(source, node);
	});
	const multiline = group.filter((node) => {
		return isMultiline(source, node);
	});

	if (singleLine.length === 0 || multiline.length === 0) {
		return null;
	}

	const desired = canReorder ? [...singleLine, ...multiline] : group;

	const replacement = desired
		.map((node, index) => {
			const previous = desired[index - 1];
			const separator = previous && (isMultiline(source, previous) || isMultiline(source, node)) ? '\n\n' : index > 0 ? '\n' : '';

			return `${separator}${nodeSource(source, node)}`;
		})
		.join('');

	const firstStart = getStart(group[0]);
	const lastEnd = getEnd(group.at(-1)!);

	if (firstStart < 0 || lastEnd < 0) {
		return null;
	}

	const start = lineStart(source, firstStart);
	const current = source.slice(start, lastEnd);

	if (current === replacement) {
		return null;
	}

	const alreadyOrdered = desired.every((node, index) => {
		return node === group[index];
	});

	if (!canReorder && !alreadyOrdered) {
		return null;
	}

	return {
		start,
		end: lastEnd,
		replacement,
	};
}

export function computeDeclarationReorderEdits(content: string, virtualName: string): Edit[] {
	const parsed = parseSync(virtualName, content) as unknown as { program: Node };
	const lists = collectStatementLists(parsed.program);
	const edits: Edit[] = [];

	for (const list of lists) {
		const importGroups = splitGroups(list, (node) => {
			return node.type === 'ImportDeclaration';
		});

		const constGroups = splitGroups(list, isConstDeclaration);

		for (const group of importGroups) {
			const edit = groupEdit(content, group, true);

			if (edit) {
				edits.push(edit);
			}
		}

		for (const group of constGroups) {
			const edit = groupEdit(content, group, canReorderConstGroup(content, group));

			if (edit) {
				edits.push(edit);
			}
		}
	}

	return edits;
}

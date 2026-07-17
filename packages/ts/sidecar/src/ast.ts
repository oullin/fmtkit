import type { Node } from '#sidecar/types';

const STATEMENT_LIST_KEYS: Record<string, 'body' | 'consequent'> = {
	Program: 'body',
	BlockStatement: 'body',
	StaticBlock: 'body',
	SwitchCase: 'consequent',
	ClassBody: 'body',
};

export function getStart(n: Node): number {
	return typeof n.start === 'number' ? n.start : (n.range?.[0] ?? -1);
}

export function getEnd(n: Node): number {
	return typeof n.end === 'number' ? n.end : (n.range?.[1] ?? -1);
}

export function visit(node: Node, fn: (n: Node) => void): void {
	fn(node);

	for (const key of Object.keys(node)) {
		const value = node[key];

		if (!value || typeof value !== 'object') {
			continue;
		}

		if (Array.isArray(value)) {
			for (const child of value) {
				if (child && typeof child === 'object' && typeof (child as Node).type === 'string') {
					visit(child as Node, fn);
				}
			}
		} else if (typeof (value as Node).type === 'string') {
			visit(value as Node, fn);
		}
	}
}

export function collectStatementLists(program: Node): Node[][] {
	const lists: Node[][] = [];

	visit(program, (n) => {
		const key = STATEMENT_LIST_KEYS[n.type];

		if (key) {
			const value = n[key];

			if (Array.isArray(value)) {
				lists.push(value as Node[]);
			}
		}

		if (n.type === 'SwitchStatement' && Array.isArray(n.cases)) {
			lists.push(n.cases as Node[]);
		}
	});

	return lists;
}

export function collectClassBodies(program: Node): Node[] {
	const bodies: Node[] = [];

	visit(program, (n) => {
		if (n.type === 'ClassBody') {
			bodies.push(n);
		}
	});

	return bodies;
}

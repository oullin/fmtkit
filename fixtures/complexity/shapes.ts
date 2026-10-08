export function ifChain(a: number, b: number, c: number): number {
	if (a > 0) {
		return 1;
	}

	if (b > 0) {
		return 2;
	}

	if (c > 0) {
		return 3;
	}

	return 0;
}

export function elseIfLadder(a: number): string {
	if (a === 1) {
		return 'one';
	} else if (a === 2) {
		return 'two';
	} else if (a === 3) {
		return 'three';
	} else {
		return 'other';
	}
}

export function switchFour(a: string): number {
	switch (a) {
		case 'a':
			return 1;
		case 'b':
			return 2;
		case 'c':
			return 3;
		case 'd':
			return 4;
		default:
			return 0;
	}
}

export function nestedClosure(): (item: number) => number {
	return (item) => {
		if (item > 0) {
			return item;
		}

		return 0;
	};
}

export function logicalRun(a: boolean, b: boolean, c: boolean, d: boolean): boolean {
	return a && b && c && d;
}

export function mixedLogical(a: boolean, b: boolean, c: boolean): boolean {
	return (a && b) || c;
}

export function loopWithIf(items: number[]): number {
	let total = 0;

	for (const item of items) {
		if (item > 0) {
			total += item;
		}
	}

	return total;
}

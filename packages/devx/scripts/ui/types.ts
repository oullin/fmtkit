export type Node = {
	type: string;
	start?: number;
	end?: number;
	range?: [number, number];
	[key: string]: unknown;
};

export type Edit = {
	start: number;
	end: number;
	replacement: string;
};

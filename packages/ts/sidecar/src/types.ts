export type { Node } from '#sidecar/node-schema';

export type Edit = {
	start: number;
	end: number;
	replacement: string;
};

const first = {
	handler(): void {},
};

const second = {
	handler(rows: number[]): number[] {
		return rows.filter((row) => row > 0 && row < 10);
	},
};

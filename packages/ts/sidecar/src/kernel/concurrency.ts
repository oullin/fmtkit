/**
 * Apply an async operation across items with a bounded worker pool.
 *
 * Results are written back at each item's original index, so the returned array
 * preserves input order regardless of completion order. At most `limit` workers
 * run concurrently, and never more than there are items.
 *
 * @param items - The inputs to process.
 * @param limit - The maximum number of concurrent operations.
 * @param operation - The async operation applied to each item.
 * @returns The operation results in input order.
 */
export async function mapPool<T, R>(items: T[], limit: number, operation: (item: T) => Promise<R>): Promise<R[]> {
	const results = new Array<R>(items.length);

	let nextIndex = 0;

	const worker = async (): Promise<void> => {
		while (true) {
			const index = nextIndex++;

			if (index >= items.length) {
				return;
			}

			const item = items[index];

			if (item !== undefined) {
				results[index] = await operation(item);
			}
		}
	};

	const workers = Array.from({ length: Math.max(1, Math.min(limit, items.length)) }, worker);

	await Promise.all(workers);

	return results;
}

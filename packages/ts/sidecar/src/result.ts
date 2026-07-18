/**
 * The Result vocabulary this slice reports expected failures with.
 *
 * `Result` is a value type discriminating on a tag, so its helpers stay
 * functional by design rather than being wrapped in a class.
 */

/**
 * A computed value that is either a success or a typed failure.
 *
 * @template T - The success type.
 * @template E - The failure type.
 */
export type Result<T, E extends Error> = { readonly _tag: 'ok'; readonly value: T } | { readonly _tag: 'err'; readonly error: E };

/**
 * Wrap a value as a success.
 *
 * @template T - The success type.
 * @param value - The value to wrap.
 * @returns A successful result carrying `value`.
 */
export function ok<T>(value: T): Result<T, never> {
	return { _tag: 'ok', value };
}

/**
 * Wrap an error as a failure.
 *
 * @template E - The error type.
 * @param error - The error to wrap.
 * @returns A failed result carrying `error`.
 */
export function err<E extends Error>(error: E): Result<never, E> {
	return { _tag: 'err', error };
}

/**
 * Narrow a result to its failure branch.
 *
 * @template T - The success type.
 * @template E - The error type.
 * @param result - The result to inspect.
 * @returns `true` when the result is a failure, narrowing `result.error`.
 */
export function isErr<T, E extends Error>(result: Result<T, E>): result is { readonly _tag: 'err'; readonly error: E } {
	return result._tag === 'err';
}

// The type of `save` comes from another file: no per-file linter can see it.
import { later, save } from './api';

// Non-ASCII text before a finding checks that ranges are byte offsets: café ✓
export async function run(ids: string[]): Promise<number[]> {
  save('a');

  for (const index in ids) {
    console.log(index);
  }

  delete ids[0];

  const total = await 42;
  const sorted = [3, 1, 2].sort();
  const negative = -'1';

  later(total);

  return [...sorted, negative];
}

export async function clean(ids: string[]): Promise<void> {
  await save(ids[0] ?? 'b');
  void later(1);
}

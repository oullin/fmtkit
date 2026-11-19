export async function save(id: string): Promise<void> {
  await Promise.resolve(id);
}

export const later = (value: number) => Promise.resolve(value);

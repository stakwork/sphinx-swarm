export function formatEnv(envsObj): { key: string; value: string }[] {
  return Object.entries(envsObj).map(([key, value]) => ({
    key,
    value: String(value),
  }));
}

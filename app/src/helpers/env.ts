export function formatEnv(envsObj) {
  return Object.entries(envsObj).map(([key, value]) => ({
    key,
    value,
  }));
}

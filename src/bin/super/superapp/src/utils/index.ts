export function splitHost(hostFullPath: string) {
  if (hostFullPath) {
    const arr = hostFullPath.split(".");
    if (arr[0]) {
      return arr[0];
    }
    return "";
  }
  return "";
}

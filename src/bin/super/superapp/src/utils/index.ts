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

export function getSwarmNumber(default_host: string) {
  // Regular expression to match the number in the string
  const match = default_host.match(/\d+/);

  if (match) {
    return match[0];
  } else {
    return "";
  }
}

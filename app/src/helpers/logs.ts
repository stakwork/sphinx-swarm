export function cleanLog(log) {
  return log.replace(/\x1B\[[0-9;]*m/g, ""); // Remove ANSI escape codes
}

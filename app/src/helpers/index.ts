export function formatSatsNumbers(num) {
  if (!num) return "0";
  const numFormat = new Intl.NumberFormat().format(num).replaceAll(",", " ");
  return numFormat;
}

export function formatMillisatsToSats(num) {
  if (!num) return 0;
  const n = typeof num === "number" ? Math.floor(num / 1000) : 0;
  formatSatsNumbers(n);
}

export function convertMillisatsToSats(num) {
  if (!num) return 0;
  const n = typeof num === "number" ? Math.floor(num / 1000) : 0;
  return n;
}

export function convertBtcToSats(num) {
  return Number(num) * 1000000000;
}

export function bufferToHexString(byteArray) {
  return Array.from(byteArray, function (byte: any) {
    return ("0" + (byte & 0xff).toString(16)).slice(-2);
  }).join("");
}

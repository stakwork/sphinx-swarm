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

export function convertSatsToMilliSats(num) {
  return Number(num) * 1000;
}

export function convertBtcToSats(num) {
  return Number(num) * 1000000000;
}

export function bufferToHexString(byteArray) {
  return Array.from(byteArray, function (byte: any) {
    return ("0" + (byte & 0xff).toString(16)).slice(-2);
  }).join("");
}

function addZeroToSingleDigit(value: number): string {
  if (value <= 9) {
    return `0${value}`;
  }
  return `${value}`;
}

export function parseDate(date: number): string {
  let newDate = new Date(date * 1000);
  const year = newDate.getFullYear();
  const month = newDate.getMonth();
  const day = newDate.getDate();
  let hours = newDate.getHours();
  if (hours === 0) {
    hours = 0;
  } else {
    hours = hours % 12;
    hours = hours ? hours : 12;
  }
  const minute = newDate.getMinutes();
  const amPm = hours >= 12 ? "PM" : "AM";
  return `${year}-${addZeroToSingleDigit(month + 1)}-${addZeroToSingleDigit(
    day
  )} ${addZeroToSingleDigit(hours)}:${addZeroToSingleDigit(minute)} ${amPm}`;
}

export function shortTransactionId(id: string): string {
  return `${id.substring(0, 4)}...${id.substring(id.length - 4, id.length)}`;
}

export function shortPubkey(id: string): string {
  return `${id.substring(0, 15)}...`;
}

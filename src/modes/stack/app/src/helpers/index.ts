export function formatSatsNumbers(num) {
  const numFormat = new Intl.NumberFormat().format(num).replaceAll(",", " ");
  return numFormat;
}

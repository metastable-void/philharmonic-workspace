export function formatTimestamp(value: number): string {
  const date = new Date(value);
  const pad = (n: number): string => String(n).padStart(2, "0");
  const offsetMin = -date.getTimezoneOffset();
  const offsetSign = offsetMin >= 0 ? "+" : "-";
  const offsetAbs = Math.abs(offsetMin);
  const offsetHH = pad(Math.floor(offsetAbs / 60));
  const offsetMM = pad(offsetAbs % 60);

  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
    `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}` +
    `${offsetSign}${offsetHH}:${offsetMM}`
  );
}

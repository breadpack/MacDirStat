const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"];

export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const exp = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), UNITS.length - 1);
  const value = bytes / Math.pow(1024, exp);
  return `${value.toFixed(exp > 0 ? 1 : 0)} ${UNITS[exp]}`;
}

export function formatNumber(n: number): string {
  return n.toLocaleString();
}

export function formatDate(timestamp: number | null): string {
  if (timestamp === null || timestamp === 0) return "-";
  const d = new Date(timestamp * 1000);
  return (
    d.toLocaleDateString() +
    " " +
    d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
  );
}

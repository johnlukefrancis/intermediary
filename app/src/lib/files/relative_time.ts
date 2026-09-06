// Path: app/src/lib/files/relative_time.ts
// Description: One formatter for "Ns/Nm/Nh/Nd ago" labels shared by the file table and the stream

export function formatRelativeTime(isoDate: string): string {
  const then = new Date(isoDate).getTime();
  if (Number.isNaN(then)) return "--";

  const diffSec = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (diffSec < 60) return `${String(diffSec)}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${String(diffMin)}m ago`;
  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) return `${String(diffHour)}h ago`;
  return `${String(Math.floor(diffHour / 24))}d ago`;
}

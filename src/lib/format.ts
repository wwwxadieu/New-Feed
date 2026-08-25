const RELATIVE = new Intl.RelativeTimeFormat("vi", { numeric: "auto" });
const NUMBER = new Intl.NumberFormat("vi-VN");

export function relativeTime(iso: string): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "";
  const minutes = Math.round((then - Date.now()) / 60000);
  if (Math.abs(minutes) < 60) return RELATIVE.format(Math.min(minutes, -1), "minute");
  const hours = Math.round(minutes / 60);
  if (Math.abs(hours) < 24) return RELATIVE.format(hours, "hour");
  return RELATIVE.format(Math.round(hours / 24), "day");
}

export function hoursSince(iso: string): number {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return Number.POSITIVE_INFINITY;
  return (Date.now() - then) / 3_600_000;
}

export function clockTime(iso: string | null): string {
  if (!iso) return "chưa làm mới";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "chưa làm mới";
  return date.toLocaleTimeString("vi-VN", { hour: "2-digit", minute: "2-digit" });
}

export const formatNumber = (value: number): string => NUMBER.format(value);

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

/** Chữ viết tắt hai ký tự cho huy hiệu nguồn, bỏ dấu tiếng Việt cho gọn. */
export function initials(name: string): string {
  const plain = name.normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/đ/gi, "d");
  const words = plain.split(/[\s.\-_]+/).filter(Boolean);
  if (words.length === 0) return "??";
  if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
  return (words[0][0] + words[1][0]).toUpperCase();
}

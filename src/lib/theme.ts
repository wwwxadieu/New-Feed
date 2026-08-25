import { useEffect, useState } from "react";

export type ThemeChoice = "auto" | "light" | "dark";

const STORAGE_KEY = "news-feed:theme";

function readStored(): ThemeChoice {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "auto") return stored;
  } catch {
    // Trình duyệt chặn lưu trữ: dùng mặc định.
  }
  return "auto";
}

/**
 * "auto" bám theo cài đặt sáng/tối của hệ điều hành và đổi ngay khi người
 * dùng đổi trong Windows mà không cần khởi động lại ứng dụng.
 */
export function useTheme() {
  const [choice, setChoice] = useState<ThemeChoice>(readStored);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved = choice === "auto" ? (media.matches ? "dark" : "light") : choice;
      document.documentElement.dataset.theme = resolved;
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [choice]);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, choice);
    } catch {
      // Không lưu được thì vẫn chạy bình thường trong phiên hiện tại.
    }
  }, [choice]);

  return { choice, setChoice };
}

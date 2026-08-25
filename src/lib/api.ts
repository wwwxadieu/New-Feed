import type { CleanedArticle, Settings, Snapshot } from "./types";
import { demoSnapshot } from "./demo";

/** Khi mở bằng `npm run dev` trong trình duyệt sẽ không có backend Rust. */
export const isDesktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

function demoDelay<T>(value: T, ms = 320): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

export const api = {
  getSnapshot: (): Promise<Snapshot> =>
    isDesktop ? call<Snapshot>("get_snapshot") : demoDelay(demoSnapshot(), 120),

  refresh: (): Promise<Snapshot> =>
    isDesktop ? call<Snapshot>("refresh") : demoDelay(demoSnapshot(), 900),

  addSource: (input: string): Promise<Snapshot> =>
    isDesktop
      ? call<Snapshot>("add_source", { input })
      : Promise.reject(new Error("Thêm nguồn chỉ hoạt động trong ứng dụng desktop.")),

  removeSource: (id: string): Promise<Snapshot> =>
    isDesktop ? call<Snapshot>("remove_source", { id }) : demoDelay(demoSnapshot()),

  setSourceEnabled: (id: string, enabled: boolean): Promise<Snapshot> =>
    isDesktop ? call<Snapshot>("set_source_enabled", { id, enabled }) : demoDelay(demoSnapshot()),

  saveSettings: (settings: Settings): Promise<Snapshot> =>
    isDesktop ? call<Snapshot>("save_settings", { settings }) : demoDelay(demoSnapshot()),

  readArticle: (url: string): Promise<CleanedArticle> =>
    isDesktop ? call<CleanedArticle>("read_article", { url }) : demoDelay(demoArticle(), 700),

  openExternal: async (url: string): Promise<void> => {
    if (isDesktop) {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } else {
      window.open(url, "_blank", "noopener");
    }
  },

  onRefreshProgress: async (
    handler: (payload: { done: number; total: number; source: string }) => void,
  ): Promise<() => void> => {
    if (!isDesktop) return () => {};
    const { listen } = await import("@tauri-apps/api/event");
    return listen<{ done: number; total: number; source: string }>("refresh:progress", (event) =>
      handler(event.payload),
    );
  },
};

function demoArticle(): CleanedArticle {
  return {
    blocks: [
      {
        kind: "paragraph",
        text: "Đây là nội dung mẫu hiển thị khi chạy trong trình duyệt. Trong ứng dụng desktop, phần này là bài viết thật đã được tải về và bóc tách: quảng cáo, popup và script theo dõi bị loại bỏ, chỉ giữ lại chữ và ảnh của bài gốc.",
      },
      {
        kind: "quote",
        text: "Bộ đếm phía trên là số khối thật sự bị loại bỏ khỏi trang gốc, do trình bóc tách đếm được chứ không phải con số minh hoạ.",
      },
      {
        kind: "paragraph",
        text: "Trình bóc tách hoạt động theo nguyên tắc của readability: xoá các thẻ không thuộc nội dung, xoá phần tử có lớp hoặc id khớp mẫu quảng cáo, rồi chọn khối có mật độ chữ cao nhất làm thân bài.",
      },
      {
        kind: "heading",
        text: "Ảnh trong bài được giữ nguyên",
      },
      {
        kind: "paragraph",
        text: "Ảnh được lấy cả từ thuộc tính lazy-load và srcset, sau đó quy về địa chỉ tuyệt đối để hiển thị đúng trong ứng dụng.",
      },
    ],
    images: [],
    leadImage: null,
    byline: "Nội dung mẫu",
    wordCount: 148,
    readMinutes: 1,
    removedAds: 14,
    removedPopups: 3,
    removedTrackers: 9,
  };
}

import type { CleanedArticle, Settings, Snapshot, Weather } from "./types";
import { demoSnapshot } from "./demo";

/** Khi mở bằng `npm run dev` trong trình duyệt sẽ không có backend Rust. */
export const isDesktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * Địa chỉ hiển thị được của một tệp trên máy. Ảnh đại diện được tải sẵn vào
 * thư mục đệm nên vẽ ra tức thì, không phải chờ gọi mạng cho từng thẻ tin.
 */
export function assetUrl(path: string): string {
  if (!isDesktop) return path;
  const internals = (window as unknown as {
    __TAURI_INTERNALS__?: { convertFileSrc?: (p: string, protocol?: string) => string };
  }).__TAURI_INTERNALS__;
  return internals?.convertFileSrc?.(path, "asset") ?? path;
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

function demoDelay<T>(value: T, ms = 320): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

/** Đăng ký nghe một sự kiện của backend, trả về hàm để huỷ đăng ký. */
async function onEvent<T>(name: string, handler: (payload: T) => void): Promise<() => void> {
  if (!isDesktop) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(name, (event) => handler(event.payload));
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

  translateTexts: (texts: string[]): Promise<string[]> =>
    isDesktop
      ? call<string[]>("translate_texts", { texts })
      : Promise.reject(new Error("Dịch chỉ hoạt động trong ứng dụng desktop.")),

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

  onRefreshProgress: (
    handler: (payload: { done: number; total: number; source: string }) => void,
  ): Promise<() => void> => onEvent("refresh:progress", handler),

  /**
   * Ảnh chụp mới do backend tự đẩy lên sau khi bổ sung xong ảnh hoặc bản dịch.
   * Lệnh `refresh` trả về ngay khi đọc xong feed, phần còn lại tới sau qua đây.
   */
  onSnapshotUpdated: (handler: (snapshot: Snapshot) => void): Promise<() => void> =>
    onEvent("snapshot:updated", handler),

  /** Việc backend đang làm ở nền, hoặc null khi đã xong. */
  onEnrichPhase: (handler: (label: string | null) => void): Promise<() => void> =>
    onEvent("enrich:phase", handler),
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
    partial: false,
  };
}

/**
 * Thời tiết hiện tại. Trả về null khi không lấy được — ô này là phần phụ nên
 * hỏng thì lặng lẽ không hiện chứ không báo lỗi.
 */
export const getWeather = (): Promise<Weather | null> =>
  isDesktop
    ? call<Weather | null>("get_weather")
    : demoDelay<Weather | null>({ tempC: 28, code: 2, isDay: true, place: "Hà Nội" }, 400);

export interface Source {
  id: string;
  title: string;
  homeUrl: string;
  feedUrl: string;
  enabled: boolean;
  addedAt: string;
  lastFetched: string | null;
  lastError: string | null;
  articleCount: number;
  logo: string | null;
  /** "vi" hoặc "other" — quyết định có cần dịch tiêu đề hay không. */
  language: string | null;
}

export interface Article {
  id: string;
  sourceId: string;
  sourceTitle: string;
  title: string;
  url: string;
  summary: string;
  published: string;
  image: string | null;
  titleVi: string | null;
  summaryVi: string | null;
  /** Đường dẫn ảnh đã tải sẵn trên máy. */
  thumb: string | null;
}

export interface Cluster {
  id: string;
  title: string;
  titleVi: string | null;
  summaryVi: string | null;
  summary: string;
  topic: string;
  newest: string;
  score: number;
  sourceCount: number;
  articles: Article[];
}

export type Block =
  | { kind: "paragraph"; text: string }
  | { kind: "heading"; text: string }
  | { kind: "quote"; text: string }
  | { kind: "image"; src: string };

export interface CleanedArticle {
  blocks: Block[];
  images: string[];
  leadImage: string | null;
  byline: string | null;
  wordCount: number;
  readMinutes: number;
  removedAds: number;
  removedPopups: number;
  removedTrackers: number;
  /** Đúng khi chỉ lấy được phần tóm tắt chứ không phải toàn văn. */
  partial: boolean;
}

export interface Settings {
  theme: "auto" | "light" | "dark";
  windowHours: number;
  maxPerSource: number;
  translate: boolean;
  translateEmail: string;
}

export interface Snapshot {
  sources: Source[];
  clusters: Cluster[];
  settings: Settings;
  articleCount: number;
  topicCounts: [string, number][];
  hourly: number[];
  lastRefresh: string | null;
  translateNotice: string | null;
}

export interface Topic {
  id: string;
  label: string;
}

export const TOPICS: Topic[] = [
  { id: "ai", label: "AI & mô hình" },
  { id: "security", label: "Bảo mật" },
  { id: "hardware", label: "Phần cứng" },
  { id: "device", label: "Điện thoại & thiết bị" },
  { id: "games", label: "Game & esports" },
  { id: "startup", label: "Startup & vốn" },
  { id: "ev", label: "Xe điện" },
  { id: "social", label: "Mạng xã hội" },
  { id: "space", label: "Không gian" },
  { id: "other", label: "Khác" },
];

export const TOPIC_LABEL: Record<string, string> = Object.fromEntries(
  TOPICS.map((t) => [t.id, t.label]),
);

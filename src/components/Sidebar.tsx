import type { Cluster, Source } from "../lib/types";
import { TOPICS } from "../lib/types";
import type { ThemeChoice } from "../lib/theme";
import { clockTime, formatNumber } from "../lib/format";
import { AutoIcon, MoonIcon, SourceIcon, SunIcon } from "./Icons";
import { TopicIcon } from "./TopicIcons";
import { SourceLogo } from "./SourceLogo";

interface Props {
  clusters: Cluster[];
  sources: Source[];
  topic: string;
  sourceId: string | null;
  onTopic: (topic: string) => void;
  onSource: (id: string | null) => void;
  onOpenSources: () => void;
  lastRefresh: string | null;
  theme: ThemeChoice;
  onTheme: (theme: ThemeChoice) => void;
  translate: boolean;
  onTranslate: (value: boolean) => void;
}

const THEME_OPTIONS: { value: ThemeChoice; label: string; icon: React.ReactNode }[] = [
  { value: "auto", label: "Theo hệ thống", icon: <AutoIcon size={14} /> },
  { value: "light", label: "Sáng", icon: <SunIcon size={14} /> },
  { value: "dark", label: "Tối", icon: <MoonIcon size={14} /> },
];

export function Sidebar({
  clusters,
  sources,
  topic,
  sourceId,
  onTopic,
  onSource,
  onOpenSources,
  lastRefresh,
  theme,
  onTheme,
  translate,
  onTranslate,
}: Props) {
  const counts = clusters.reduce<Record<string, number>>((acc, cluster) => {
    acc[cluster.topic] = (acc[cluster.topic] ?? 0) + 1;
    return acc;
  }, {});

  const visibleTopics = TOPICS.filter((t) => (counts[t.id] ?? 0) > 0);
  const enabled = sources.filter((s) => s.enabled);
  // Dấu hiệu nguồn hỏng trước đây nằm ở panel bên phải; panel đó đã bỏ nên
  // đưa xuống dòng trạng thái để tín hiệu này không mất hẳn.
  const failing = enabled.filter((s) => s.lastError).length;

  return (
    <aside className="sidebar">
      {/* Chỉ phần này cuộn. Khối Giao diện nằm ngoài nên luôn thấy, kể cả khi
          danh sách nguồn dài hơn chiều cao cửa sổ. */}
      <div className="sidebar-scroll">
      <nav className="nav-section" aria-label="Chủ đề">
        <span className="section-title">Chủ đề</span>
        <button
          className="nav-item"
          aria-current={topic === "all" && sourceId === null}
          onClick={() => onTopic("all")}
        >
          <span className="nav-icon">
            <TopicIcon topic="all" />
          </span>
          Tất cả
          <span className="count">{clusters.length}</span>
        </button>
        {visibleTopics.map((item) => (
          <button
            key={item.id}
            className="nav-item"
            aria-current={topic === item.id && sourceId === null}
            onClick={() => onTopic(item.id)}
          >
            <span className="nav-icon">
              <TopicIcon topic={item.id} />
            </span>
            {item.label}
            <span className="count">{counts[item.id]}</span>
          </button>
        ))}
      </nav>

      <hr className="rail-divider" />

      <nav className="nav-section" aria-label="Nguồn tin">
        <span className="section-title">Nguồn tin</span>
        {enabled.length === 0 ? (
          <p className="rail-empty">Chưa có nguồn nào đang bật.</p>
        ) : (
          enabled.map((source) => (
            <button
              key={source.id}
              className="nav-item"
              aria-current={sourceId === source.id}
              title={source.title}
              onClick={() => onSource(sourceId === source.id ? null : source.id)}
            >
              <span className="nav-icon">
                <SourceLogo sourceId={source.id} name={source.title} size={16} radius={5} />
              </span>
              <span className="nav-label">{source.title}</span>
              <span className="count">{formatNumber(source.articleCount)}</span>
            </button>
          ))
        )}
      </nav>
      </div>

      <div className="sidebar-foot">
        <div className="nav-section" aria-label="Giao diện">
          <span className="section-title">Giao diện</span>
          <div className="segmented" style={{ margin: "0 4px 8px" }}>
            {THEME_OPTIONS.map((option) => (
              <button
                key={option.value}
                aria-selected={theme === option.value}
                aria-label={option.label}
                title={option.label}
                onClick={() => onTheme(option.value)}
                style={{
                  flex: 1,
                  display: "grid",
                  placeItems: "center",
                  color: theme === option.value ? "var(--blue)" : undefined,
                  background: theme === option.value ? "var(--glass-strong)" : undefined,
                  borderRadius: 7,
                }}
              >
                {option.icon}
              </button>
            ))}
          </div>

          <div className="toggle-row">
            <span>
              Dịch nguồn nước ngoài
              <small>Tự dịch tiêu đề tiếng nước ngoài sang tiếng Việt</small>
            </span>
            <button
              className="switch small"
              role="switch"
              aria-checked={translate}
              aria-label="Dịch nguồn nước ngoài"
              onClick={() => onTranslate(!translate)}
            >
              <i />
            </button>
          </div>
        </div>

        <button className="ghost-button" onClick={onOpenSources}>
          <SourceIcon />
          Quản lý nguồn ({sources.length})
        </button>

        <div className={`status-line${failing > 0 ? " warn" : ""}`}>
          <span className="pulse-dot" />
          {failing > 0 ? (
            <>
              {failing} nguồn lỗi · {clockTime(lastRefresh)}
            </>
          ) : (
            <>Cập nhật lúc {clockTime(lastRefresh)}</>
          )}
        </div>
      </div>
    </aside>
  );
}

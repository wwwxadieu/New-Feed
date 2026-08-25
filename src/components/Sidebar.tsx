import type { Cluster } from "../lib/types";
import { TOPICS } from "../lib/types";
import type { ThemeChoice } from "../lib/theme";
import { clockTime } from "../lib/format";
import { AutoIcon, MoonIcon, SourceIcon, SunIcon } from "./Icons";

interface Props {
  clusters: Cluster[];
  topic: string;
  onTopic: (topic: string) => void;
  onOpenSources: () => void;
  sourceCount: number;
  lastRefresh: string | null;
  theme: ThemeChoice;
  onTheme: (theme: ThemeChoice) => void;
}

const THEME_OPTIONS: { value: ThemeChoice; label: string; icon: React.ReactNode }[] = [
  { value: "auto", label: "Theo hệ thống", icon: <AutoIcon size={14} /> },
  { value: "light", label: "Sáng", icon: <SunIcon size={14} /> },
  { value: "dark", label: "Tối", icon: <MoonIcon size={14} /> },
];

export function Sidebar({
  clusters,
  topic,
  onTopic,
  onOpenSources,
  sourceCount,
  lastRefresh,
  theme,
  onTheme,
}: Props) {
  const counts = clusters.reduce<Record<string, number>>((acc, cluster) => {
    acc[cluster.topic] = (acc[cluster.topic] ?? 0) + 1;
    return acc;
  }, {});

  const visibleTopics = TOPICS.filter((t) => (counts[t.id] ?? 0) > 0);

  return (
    <aside className="sidebar">
      <nav className="nav-section" aria-label="Chủ đề">
        <span className="section-title">Chủ đề</span>
        <button className="nav-item" aria-current={topic === "all"} onClick={() => onTopic("all")}>
          <span className="swatch" />
          Tất cả
          <span className="count">{clusters.length}</span>
        </button>
        {visibleTopics.map((item) => (
          <button
            key={item.id}
            className="nav-item"
            aria-current={topic === item.id}
            onClick={() => onTopic(item.id)}
          >
            <span className="swatch" />
            {item.label}
            <span className="count">{counts[item.id]}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar-foot">
        <div className="nav-section" aria-label="Giao diện">
          <span className="section-title">Giao diện</span>
          <div className="segmented" style={{ margin: "0 4px" }}>
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
        </div>

        <button className="ghost-button" onClick={onOpenSources}>
          <SourceIcon />
          Quản lý nguồn ({sourceCount})
        </button>

        <div className="status-line">
          <span className="pulse-dot" />
          Cập nhật lúc {clockTime(lastRefresh)}
        </div>
      </div>
    </aside>
  );
}

import { useCallback, useEffect, useMemo, useState } from "react";
import type { Cluster, Snapshot } from "./lib/types";
import { api, isDesktop } from "./lib/api";
import { useTheme } from "./lib/theme";
import { formatNumber, hoursSince } from "./lib/format";
import { Segmented } from "./components/Segmented";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { ClusterCard } from "./components/ClusterCard";
import { InsightRail } from "./components/InsightRail";
import { ReaderSheet } from "./components/ReaderSheet";
import { SourceManager } from "./components/SourceManager";
import { PlusIcon, RefreshIcon, SearchIcon } from "./components/Icons";

type SortKey = "hot" | "sources" | "new";

const WINDOWS = [
  { value: "6", label: "6 giờ" },
  { value: "24", label: "24 giờ" },
  { value: "168", label: "7 ngày" },
] as const;

const SORTS = [
  { value: "hot" as const, label: "Đang nóng" },
  { value: "sources" as const, label: "Nhiều nguồn" },
  { value: "new" as const, label: "Mới nhất" },
];

export default function App() {
  const { choice: theme, setChoice: setTheme } = useTheme();

  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [topic, setTopic] = useState("all");
  const [sort, setSort] = useState<SortKey>("hot");
  const [windowHours, setWindowHours] = useState("24");
  const [query, setQuery] = useState("");

  const [reader, setReader] = useState<Cluster | null>(null);
  const [sourcesOpen, setSourcesOpen] = useState(false);

  const [refreshing, setRefreshing] = useState(false);
  const [addingSource, setAddingSource] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [toast, setToast] = useState<{ message: string; error?: boolean } | null>(null);

  const notify = useCallback((message: string, error = false) => {
    setToast({ message, error });
    window.setTimeout(() => setToast(null), error ? 6000 : 3200);
  }, []);

  useEffect(() => {
    api.getSnapshot().then(setSnapshot).catch((err: unknown) => {
      notify(err instanceof Error ? err.message : String(err), true);
    });
  }, [notify]);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    api.onRefreshProgress(({ done, total }) => setProgress({ done, total })).then((off) => {
      dispose = off;
    });
    return () => dispose?.();
  }, []);

  const refresh = useCallback(async () => {
    if (refreshing) return;
    setRefreshing(true);
    setProgress({ done: 0, total: snapshot?.sources.filter((s) => s.enabled).length ?? 1 });
    try {
      const next = await api.refresh();
      setSnapshot(next);
      const failed = next.sources.filter((s) => s.enabled && s.lastError).length;
      notify(
        failed > 0
          ? `Đã làm mới. ${failed} nguồn không tải được — xem chi tiết trong Quản lý nguồn.`
          : `Đã làm mới ${next.sources.filter((s) => s.enabled).length} nguồn.`,
        failed > 0,
      );
    } catch (err: unknown) {
      notify(err instanceof Error ? err.message : String(err), true);
    } finally {
      setRefreshing(false);
      setProgress(null);
    }
  }, [refreshing, snapshot, notify]);

  // Làm mới ngay lần mở đầu tiên nếu kho tin còn trống.
  useEffect(() => {
    if (isDesktop && snapshot && snapshot.articleCount === 0 && snapshot.sources.length > 0 && !refreshing) {
      void refresh();
    }
    // Chỉ chạy khi ảnh chụp trạng thái lần đầu về tới.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [snapshot?.sources.length]);

  const addSource = useCallback(
    async (input: string) => {
      setAddingSource(true);
      try {
        const next = await api.addSource(input);
        setSnapshot(next);
        notify("Đã thêm nguồn và tải bài đầu tiên.");
      } catch (err: unknown) {
        notify(err instanceof Error ? err.message : String(err), true);
      } finally {
        setAddingSource(false);
      }
    },
    [notify],
  );

  const removeSource = useCallback(
    (id: string) => {
      api.removeSource(id).then(setSnapshot).catch((err: unknown) => {
        notify(err instanceof Error ? err.message : String(err), true);
      });
    },
    [notify],
  );

  const toggleSource = useCallback(
    (id: string, enabled: boolean) => {
      api.setSourceEnabled(id, enabled).then(setSnapshot).catch((err: unknown) => {
        notify(err instanceof Error ? err.message : String(err), true);
      });
    },
    [notify],
  );

  const clusters = useMemo(() => {
    if (!snapshot) return [];
    const limit = Number(windowHours);
    const needle = query.trim().toLowerCase();

    const filtered = snapshot.clusters.filter((cluster) => {
      if (hoursSince(cluster.newest) > limit) return false;
      if (topic !== "all" && cluster.topic !== topic) return false;
      if (!needle) return true;
      return (
        cluster.title.toLowerCase().includes(needle) ||
        cluster.summary.toLowerCase().includes(needle) ||
        cluster.articles.some((a) => a.sourceTitle.toLowerCase().includes(needle))
      );
    });

    const sorted = [...filtered];
    if (sort === "sources") sorted.sort((a, b) => b.sourceCount - a.sourceCount);
    else if (sort === "new") sorted.sort((a, b) => Date.parse(b.newest) - Date.parse(a.newest));
    else sorted.sort((a, b) => b.score - a.score);
    return sorted;
  }, [snapshot, topic, sort, windowHours, query]);

  const windowLabel = WINDOWS.find((w) => w.value === windowHours)?.label ?? "24 giờ";
  const articlesInView = clusters.reduce((sum, c) => sum + c.articles.length, 0);

  return (
    <>
      <div className="ambient" aria-hidden="true">
        <span />
        <span />
        <span />
      </div>

      <div className="window">
        <TitleBar />

        <div className="shell">
          {snapshot && (
            <Sidebar
              clusters={snapshot.clusters}
              topic={topic}
              onTopic={setTopic}
              onOpenSources={() => setSourcesOpen(true)}
              sourceCount={snapshot.sources.length}
              lastRefresh={snapshot.lastRefresh}
              theme={theme}
              onTheme={setTheme}
            />
          )}

          <main className="main">
            <div className="toolbar">
              <label className="searchfield">
                <SearchIcon />
                <input
                  type="search"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="Tìm theo sự kiện, từ khoá, nguồn…"
                  aria-label="Tìm kiếm"
                />
              </label>

              <Segmented
                label="Khoảng thời gian"
                options={WINDOWS.map((w) => ({ value: w.value, label: w.label }))}
                value={windowHours}
                onChange={setWindowHours}
              />

              <div style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
                <button
                  className="icon-button"
                  onClick={() => setSourcesOpen(true)}
                  title="Thêm nguồn tin"
                  aria-label="Thêm nguồn tin"
                >
                  <PlusIcon size={16} />
                </button>
                <button
                  className={`icon-button${refreshing ? " spinning" : ""}`}
                  onClick={refresh}
                  disabled={refreshing}
                  title="Làm mới"
                  aria-label="Làm mới"
                >
                  <RefreshIcon />
                </button>
              </div>
            </div>

            <div className="progress-strip">
              {progress && progress.total > 0 && (
                <i style={{ width: `${Math.round((progress.done / progress.total) * 100)}%` }} />
              )}
            </div>

            <div className="feed">
              <div className="feed-head">
                <div>
                  <h1>Tổng quan</h1>
                  <p>
                    Gộp <b>{formatNumber(articlesInView)}</b> bài thành <b>{formatNumber(clusters.length)}</b> cụm sự
                    kiện trong {windowLabel.toLowerCase()} qua
                  </p>
                </div>
                <div style={{ marginLeft: "auto" }}>
                  <Segmented label="Sắp xếp" options={SORTS} value={sort} onChange={setSort} />
                </div>
              </div>

              <div className="cluster-list view-swap" key={`${topic}-${sort}-${windowHours}`}>
                {clusters.length === 0 ? (
                  <div className="empty-state">
                    <h2>{snapshot ? "Chưa có cụm tin nào ở bộ lọc này" : "Đang tải…"}</h2>
                    <p>
                      {snapshot?.sources.length
                        ? "Thử nới khoảng thời gian, xoá từ khoá tìm kiếm, hoặc bấm Làm mới để tải tin mới."
                        : "Thêm vài nguồn tin công nghệ để bắt đầu."}
                    </p>
                    {snapshot && (
                      <button className="link-button" onClick={() => setSourcesOpen(true)}>
                        <PlusIcon />
                        Thêm nguồn tin
                      </button>
                    )}
                  </div>
                ) : (
                  clusters.map((cluster, index) => (
                    <ClusterCard
                      key={cluster.id}
                      cluster={cluster}
                      index={index}
                      lead={index === 0}
                      onOpen={setReader}
                    />
                  ))
                )}
              </div>
            </div>
          </main>

          {snapshot && <InsightRail snapshot={snapshot} clusters={clusters} />}
        </div>
      </div>

      {reader && <ReaderSheet cluster={reader} onClose={() => setReader(null)} />}

      {sourcesOpen && snapshot && (
        <SourceManager
          sources={snapshot.sources}
          busy={addingSource}
          onAdd={addSource}
          onRemove={removeSource}
          onToggle={toggleSource}
          onClose={() => setSourcesOpen(false)}
        />
      )}

      {toast && (
        <div className={`toast${toast.error ? " error" : ""}`} role="status">
          <span className="dot" />
          {toast.message}
        </div>
      )}
    </>
  );
}

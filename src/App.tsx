import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Cluster, Snapshot } from "./lib/types";
import { api, isDesktop } from "./lib/api";
import { useTheme } from "./lib/theme";
import { useElasticScroll } from "./lib/elasticScroll";
import { formatNumber, hoursSince } from "./lib/format";
import { Segmented } from "./components/Segmented";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { ClusterCard } from "./components/ClusterCard";
import { HotStrip } from "./components/HotStrip";
import { FEATURE_COUNT, MAGAZINE_MIN, MagazineHead } from "./components/MagazineHead";
import { ReaderSheet } from "./components/ReaderSheet";
import { SourceManager } from "./components/SourceManager";
import { SourcesContext } from "./components/SourceLogo";
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

/** Lần làm mới gần nhất cũ hơn ngần này thì tự làm mới lại. */
const STALE_AFTER_MS = 5 * 60 * 1000;
/** Nhịp kiểm tra tin mới trong lúc ứng dụng đang mở. */
const AUTO_REFRESH_MS = 10 * 60 * 1000;
/**
 * Số cụm dựng sẵn mỗi lượt. Cuộn tới cuối thì nạp thêm bấy nhiêu nữa: dựng
 * cả vài trăm thẻ tin ngay từ đầu làm lượt vẽ đầu tiên chậm hẳn, mà người
 * đọc gần như không bao giờ cuộn hết chỗ đó.
 */
const PAGE_SIZE = 30;
/**
 * Trần chờ cho một lượt làm mới.
 *
 * Cờ "đang làm mới" chặn lượt sau chồng lên lượt trước, nên nếu backend không
 * bao giờ trả lời thì cờ đó kẹt lại vĩnh viễn: nút làm mới câm và nhịp tự
 * động cũng đứng, cho tới khi mở lại ứng dụng. Thà báo hỏng còn hơn treo im.
 */
const REFRESH_TIMEOUT_MS = 120 * 1000;

export default function App() {
  const { choice: theme, setChoice: setTheme } = useTheme();

  const feedRef = useRef<HTMLElement>(null);
  const feedInnerRef = useRef<HTMLDivElement>(null);
  useElasticScroll(feedRef, feedInnerRef);

  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [topic, setTopic] = useState("all");
  const [sourceId, setSourceId] = useState<string | null>(null);
  const [sort, setSort] = useState<SortKey>("hot");
  const [windowHours, setWindowHours] = useState("24");
  const [query, setQuery] = useState("");

  const [reader, setReader] = useState<Cluster | null>(null);
  const [sourcesOpen, setSourcesOpen] = useState(false);

  const [refreshing, setRefreshing] = useState(false);
  const [addingSource, setAddingSource] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [toast, setToast] = useState<{ message: string; error?: boolean } | null>(null);
  /** Việc backend đang làm ở nền sau khi danh sách tin đã hiện ra. */
  const [phase, setPhase] = useState<string | null>(null);
  /** Số cụm đang được dựng; tăng dần khi cuộn tới cuối. */
  const [visible, setVisible] = useState(PAGE_SIZE);

  const sentinelRef = useRef<HTMLDivElement>(null);
  // Bản mới nhất để các bộ hẹn giờ đọc mà không phải dựng lại theo mỗi lần
  // trạng thái đổi — nếu không, mỗi lượt bổ sung nền sẽ đặt lại đồng hồ.
  const snapshotRef = useRef<Snapshot | null>(null);
  const refreshingRef = useRef(false);
  snapshotRef.current = snapshot;

  const pickTopic = useCallback((next: string) => {
    setTopic(next);
    setSourceId(null);
  }, []);

  const pickSource = useCallback((next: string | null) => {
    setSourceId(next);
    setTopic("all");
  }, []);

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

  // Ảnh và bản dịch về sau lượt đọc feed, backend tự đẩy lên khi xong.
  useEffect(() => {
    const disposers: (() => void)[] = [];
    let cancelled = false;
    const keep = (off: () => void) => (cancelled ? off() : disposers.push(off));

    api
      .onSnapshotUpdated((next) => {
        setSnapshot(next);
        if (next.translateNotice) notify(next.translateNotice, true);
      })
      .then(keep);
    api.onEnrichPhase(setPhase).then(keep);

    return () => {
      cancelled = true;
      disposers.forEach((off) => off());
    };
  }, [notify]);

  const refresh = useCallback(
    // Lượt tự động chạy im lặng: chỉ báo khi có nguồn hỏng, còn lại không
    // bắn thông báo để không quấy người đang đọc.
    async (silent = false) => {
      if (refreshingRef.current) return;
      refreshingRef.current = true;
      setRefreshing(true);
      setProgress({ done: 0, total: snapshotRef.current?.sources.filter((s) => s.enabled).length ?? 1 });

      let guard = 0;
      const deadline = new Promise<never>((_, reject) => {
        guard = window.setTimeout(
          () => reject(new Error("Lượt làm mới không phản hồi. Xem Quản lý nguồn để biết nguồn nào đang hỏng.")),
          REFRESH_TIMEOUT_MS,
        );
      });

      try {
        const next = await Promise.race([api.refresh(), deadline]);
        setSnapshot(next);
        if (next.translateNotice) notify(next.translateNotice, true);
        const failed = next.sources.filter((s) => s.enabled && s.lastError).length;
        if (failed > 0) {
          notify(`Đã làm mới. ${failed} nguồn không tải được — xem chi tiết trong Quản lý nguồn.`, true);
        } else if (!silent) {
          notify(`Đã làm mới ${next.sources.filter((s) => s.enabled).length} nguồn.`);
        }
      } catch (err: unknown) {
        if (!silent) notify(err instanceof Error ? err.message : String(err), true);
      } finally {
        window.clearTimeout(guard);
        refreshingRef.current = false;
        setRefreshing(false);
        setProgress(null);
      }
    },
    [notify],
  );

  /** Làm mới nếu tin đã cũ. Dùng chung cho lúc mở app, nhịp định kỳ và lúc
   *  cửa sổ được quay lại sau một lúc bỏ đó. */
  const refreshIfStale = useCallback(() => {
    const snap = snapshotRef.current;
    if (!isDesktop || refreshingRef.current || !snap) return;
    if (!snap.sources.some((s) => s.enabled)) return;
    const age = snap.lastRefresh ? Date.now() - Date.parse(snap.lastRefresh) : Number.POSITIVE_INFINITY;
    // So sánh thuận để mốc thời gian hỏng (NaN) rơi vào nhánh không làm mới.
    if (snap.articleCount > 0 && !(age > STALE_AFTER_MS)) return;
    void refresh(true);
  }, [refresh]);

  const staleRef = useRef(refreshIfStale);
  staleRef.current = refreshIfStale;

  // Tự làm mới ngay lần đầu ảnh chụp về tới, nếu tin đã cũ hoặc kho còn trống.
  const bootstrapped = useRef(false);
  useEffect(() => {
    if (!snapshot || bootstrapped.current) return;
    bootstrapped.current = true;
    staleRef.current();
  }, [snapshot]);

  // Nhịp định kỳ trong lúc app đang mở, và mỗi lần cửa sổ được quay lại.
  useEffect(() => {
    if (!isDesktop) return;
    const check = () => staleRef.current();
    const timer = window.setInterval(check, AUTO_REFRESH_MS);
    const onVisible = () => {
      if (document.visibilityState === "visible") check();
    };
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", check);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", check);
    };
  }, []);

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

  const toggleTranslate = useCallback(
    (value: boolean) => {
      if (!snapshot) return;
      const next = { ...snapshot.settings, translate: value };
      setSnapshot({ ...snapshot, settings: next });
      api.saveSettings(next).then(setSnapshot).catch((err: unknown) => {
        notify(err instanceof Error ? err.message : String(err), true);
      });
    },
    [snapshot, notify],
  );

  const clusters = useMemo(() => {
    if (!snapshot) return [];
    const limit = Number(windowHours);
    const needle = query.trim().toLowerCase();

    const filtered = snapshot.clusters.filter((cluster) => {
      if (hoursSince(cluster.newest) > limit) return false;
      if (topic !== "all" && cluster.topic !== topic) return false;
      if (sourceId && !cluster.articles.some((a) => a.sourceId === sourceId)) return false;
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
  }, [snapshot, topic, sourceId, sort, windowHours, query]);

  const sourceMap = useMemo(
    () => new Map((snapshot?.sources ?? []).map((source) => [source.id, source])),
    [snapshot],
  );

  // Đổi bộ lọc thì quay lại trang đầu và kéo dòng tin lên đầu.
  useEffect(() => {
    setVisible(PAGE_SIZE);
    if (feedRef.current) feedRef.current.scrollTop = 0;
  }, [topic, sourceId, sort, windowHours, query]);

  const shown = useMemo(() => clusters.slice(0, visible), [clusters, visible]);

  // Cuộn gần tới cuối thì dựng thêm một trang nữa.
  //
  // Quan sát được dựng lại sau mỗi lần nạp thêm: IntersectionObserver chỉ báo
  // khi trạng thái giao nhau thay đổi, mà cột mốc thường vẫn còn trong tầm
  // nhìn sau khi trang mới vẽ xong — không dựng lại thì nó im luôn.
  useEffect(() => {
    const root = feedRef.current;
    const mark = sentinelRef.current;
    if (!root || !mark) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible((count) => Math.min(count + PAGE_SIZE, clusters.length));
        }
      },
      // Nạp trước khi người đọc chạm đáy thật, để không thấy khoảng trống.
      { root, rootMargin: "900px 0px" },
    );
    observer.observe(mark);
    return () => observer.disconnect();
  }, [visible, clusters.length]);

  // Bất kỳ tấm trượt nào đang mở thì nội dung phía sau đều phải lùi lại.
  const sheetOpen = reader !== null || sourcesOpen;
  const windowLabel = WINDOWS.find((w) => w.value === windowHours)?.label ?? "24 giờ";
  const articlesInView = clusters.reduce((sum, c) => sum + c.articles.length, 0);
  // Ít tin quá thì không đủ để dựng phân cấp, quay về lưới thường.
  const magazine = shown.length >= MAGAZINE_MIN;

  return (
    <SourcesContext.Provider value={sourceMap}>
      <div className="ambient" aria-hidden="true" />

      <div className={`window${sheetOpen ? " behind-sheet" : ""}`} aria-hidden={sheetOpen}>
        <TitleBar />

        <div className="shell">
          {snapshot && (
            <Sidebar
              clusters={snapshot.clusters}
              sources={snapshot.sources}
              topic={topic}
              sourceId={sourceId}
              onTopic={pickTopic}
              onSource={pickSource}
              onOpenSources={() => setSourcesOpen(true)}
              lastRefresh={snapshot.lastRefresh}
              theme={theme}
              onTheme={setTheme}
              translate={snapshot.settings.translate}
              onTranslate={toggleTranslate}
            />
          )}

          <div className="content">
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

              <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 4 }}>
                {phase && (
                  <span className="work-chip" title="Danh sách tin đã sẵn sàng, phần này đang được bổ sung ở nền">
                    <i />
                    {phase}
                  </span>
                )}
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
                  onClick={() => void refresh()}
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

            <main className="feed" ref={feedRef}>
              <div className="feed-inner" ref={feedInnerRef}>
                <HotStrip clusters={clusters} onOpen={setReader} />

                <div className="feed-head">
                  <div>
                    <h1>{sourceId ? (sourceMap.get(sourceId)?.title ?? "Tổng quan") : "Tổng quan"}</h1>
                    <p>
                      Gộp <b>{formatNumber(articlesInView)}</b> bài thành <b>{formatNumber(clusters.length)}</b> cụm sự
                      kiện trong {windowLabel.toLowerCase()} qua
                    </p>
                  </div>
                  <div style={{ marginLeft: "auto" }}>
                    <Segmented label="Sắp xếp" options={SORTS} value={sort} onChange={setSort} />
                  </div>
                </div>

                <div className="cluster-list view-swap" key={`${topic}-${sourceId ?? ""}-${sort}-${windowHours}`}>
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
                  ) : magazine ? (
                    <>
                      <MagazineHead
                        hero={shown[0]}
                        features={shown.slice(1, 1 + FEATURE_COUNT)}
                        onOpen={setReader}
                      />
                      {shown.slice(1 + FEATURE_COUNT).map((cluster, index) => (
                        <ClusterCard
                          key={cluster.id}
                          cluster={cluster}
                          index={index}
                          onOpen={setReader}
                        />
                      ))}
                    </>
                  ) : (
                    shown.map((cluster, index) => (
                      <ClusterCard
                        key={cluster.id}
                        cluster={cluster}
                        index={index}
                        onOpen={setReader}
                      />
                    ))
                  )}
                </div>

                {visible < clusters.length ? (
                  <div className="feed-more" ref={sentinelRef} role="status">
                    <span className="feed-more-dot" />
                    Đang dựng thêm tin…
                  </div>
                ) : (
                  clusters.length > PAGE_SIZE && (
                    <p className="feed-end">
                      Hết {formatNumber(clusters.length)} cụm tin trong {windowLabel.toLowerCase()} qua
                    </p>
                  )
                )}
              </div>
            </main>
          </div>

        </div>
      </div>

      {reader && <ReaderSheet cluster={reader} onClose={() => setReader(null)} />}

      {sourcesOpen && snapshot && (
        <SourceManager
          sources={snapshot.sources}
          busy={addingSource}
          translateEmail={snapshot.settings.translateEmail}
          onTranslateEmail={(email) => {
            const next = { ...snapshot.settings, translateEmail: email };
            api.saveSettings(next).then(setSnapshot).catch((err: unknown) => {
              notify(err instanceof Error ? err.message : String(err), true);
            });
          }}
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
    </SourcesContext.Provider>
  );
}

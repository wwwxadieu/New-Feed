import { useEffect, useRef, useState } from "react";
import type { CleanedArticle, Cluster } from "../lib/types";
import { TOPIC_LABEL } from "../lib/types";
import { api } from "../lib/api";
import { useElasticScroll } from "../lib/elasticScroll";
import { hostOf, relativeTime } from "../lib/format";
import { SourceLogo } from "./SourceLogo";
import { CloseIcon, ExternalIcon } from "./Icons";

interface Props {
  cluster: Cluster;
  onClose: () => void;
}

export function ReaderSheet({ cluster, onClose }: Props) {
  const [selected, setSelected] = useState(0);
  const [content, setContent] = useState<CleanedArticle | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Bản dịch của bài đang mở, giữ trong phiên để bấm qua lại không dịch lại.
  const [translation, setTranslation] = useState<Record<number, string> | null>(null);
  const [translating, setTranslating] = useState(false);
  const [translateError, setTranslateError] = useState<string | null>(null);
  const [showOriginal, setShowOriginal] = useState(false);

  const scrollRef = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  useElasticScroll(scrollRef, gridRef);

  const article = cluster.articles[selected] ?? cluster.articles[0];

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setContent(null);
    setTranslation(null);
    setTranslateError(null);
    setShowOriginal(false);

    api
      .readArticle(article.url)
      .then((result) => {
        if (cancelled) return;
        // Bài quá ngắn thường là dấu hiệu bóc tách trượt khối nội dung.
        if (result.blocks.length === 0) {
          setError("Không bóc tách được nội dung bài này. Hãy mở bản gốc để đọc.");
        } else {
          setContent(result);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [article.url]);

  const runTranslate = async () => {
    if (!content || translating) return;
    setTranslating(true);
    setTranslateError(null);
    try {
      // Chỉ gửi phần chữ; ảnh giữ nguyên vị trí theo chỉ số của khối.
      const indexes: number[] = [];
      const texts: string[] = [];
      content.blocks.forEach((block, index) => {
        if (block.kind !== "image") {
          indexes.push(index);
          texts.push(block.text);
        }
      });
      const done = await api.translateTexts([article.title, ...texts]);
      const map: Record<number, string> = { [-1]: done[0] };
      indexes.forEach((blockIndex, i) => {
        map[blockIndex] = done[i + 1];
      });
      setTranslation(map);
    } catch (err: unknown) {
      setTranslateError(err instanceof Error ? err.message : String(err));
    } finally {
      setTranslating(false);
    }
  };

  const usingTranslation = translation !== null && !showOriginal;
  const displayTitle = usingTranslation ? (translation[-1] ?? article.title) : article.title;

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="sheet" role="dialog" aria-modal="true" aria-label={cluster.title}>
        <div className="sheet-bar">
          <button className="icon-button" onClick={onClose} aria-label="Đóng">
            <CloseIcon />
          </button>
          <span className="pill">{TOPIC_LABEL[cluster.topic] ?? "Khác"}</span>
          <span className="stamp">{relativeTime(article.published)}</span>
          <span style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 10 }}>
            {content &&
              (translation ? (
                <button className="text-button" onClick={() => setShowOriginal((v) => !v)}>
                  {showOriginal ? "Xem bản dịch" : "Xem nguyên văn"}
                </button>
              ) : (
                <button className="text-button" onClick={runTranslate} disabled={translating}>
                  {translating ? "Đang dịch…" : "Dịch sang tiếng Việt"}
                </button>
              ))}
            <span style={{ fontSize: 11, color: "var(--label-3)" }}>Esc để đóng</span>
          </span>
        </div>

        <div className="sheet-body" ref={scrollRef}>
          <div className="reader-grid" ref={gridRef}>
            <article>
              <h1 className="reader-title">{displayTitle}</h1>
              <div className="reader-byline">
                <SourceLogo sourceId={article.sourceId} name={article.sourceTitle} size={18} />
                <span className="src">{article.sourceTitle}</span>
                <span>·</span>
                <span>{hostOf(article.url)}</span>
                {content?.readMinutes ? (
                  <>
                    <span>·</span>
                    <span>{content.readMinutes} phút đọc</span>
                  </>
                ) : null}
              </div>

              {loading && (
                <div className="loading-lines" aria-live="polite">
                  <span />
                  <span />
                  <span />
                  <span />
                </div>
              )}

              {error && (
                <p style={{ fontSize: 14, color: "var(--label-2)" }}>
                  {error}
                  {article.summary ? ` — Tóm tắt từ feed: ${article.summary}` : ""}
                </p>
              )}

              {translateError && (
                <p className="translate-error">{translateError}</p>
              )}

              {content && (
                <div className="reader-body">
                  {content.blocks.map((block, index) => {
                    const text = block.kind === "image" ? "" : (usingTranslation ? translation[index] : undefined) ?? block.text;
                    switch (block.kind) {
                      case "heading":
                        return <h3 key={index}>{text}</h3>;
                      case "quote":
                        return <blockquote key={index}>{text}</blockquote>;
                      case "image":
                        return (
                          <figure key={index}>
                            <img src={block.src} alt="" loading="lazy" />
                          </figure>
                        );
                      default:
                        return <p key={index}>{text}</p>;
                    }
                  })}
                </div>
              )}

              <button className="link-button" onClick={() => api.openExternal(article.url)} style={{ marginTop: 18 }}>
                <ExternalIcon />
                Mở bài gốc
              </button>
            </article>

            <aside className="reader-aside">
              <div className="block">
                <h4>Cùng sự kiện · {cluster.sourceCount} nguồn</h4>
                {cluster.articles.map((item, index) => (
                  <button
                    key={item.id}
                    className="src-option"
                    aria-pressed={index === selected}
                    onClick={() => setSelected(index)}
                    title={item.title}
                  >
                    <SourceLogo sourceId={item.sourceId} name={item.sourceTitle} />
                    <span
                      style={{
                        minWidth: 0,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {item.sourceTitle}
                    </span>
                    <span className="when">{relativeTime(item.published)}</span>
                  </button>
                ))}
              </div>

              {cluster.articles.length > 1 && (
                <div className="block">
                  <h4>Khác biệt tiêu đề</h4>
                  {cluster.articles.slice(0, 4).map((item) => (
                    <div key={item.id} style={{ padding: "8px 0", borderBottom: "1px solid var(--hairline)" }}>
                      <span style={{ display: "block", fontSize: 10.5, color: "var(--label-3)", marginBottom: 3 }}>
                        {item.sourceTitle}
                      </span>
                      <span style={{ fontSize: 12.5, lineHeight: 1.45, color: "var(--label-2)" }}>{item.title}</span>
                    </div>
                  ))}
                </div>
              )}
            </aside>
          </div>
        </div>
      </div>
    </div>
  );
}

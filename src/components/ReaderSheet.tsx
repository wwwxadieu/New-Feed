import { useEffect, useState } from "react";
import type { CleanedArticle, Cluster } from "../lib/types";
import { TOPIC_LABEL } from "../lib/types";
import { api } from "../lib/api";
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

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="sheet" role="dialog" aria-modal="true" aria-label={cluster.title}>
        <div className="sheet-bar">
          <button className="icon-button" onClick={onClose} aria-label="Đóng">
            <CloseIcon />
          </button>
          <span className="pill">{TOPIC_LABEL[cluster.topic] ?? "Khác"}</span>
          <span className="stamp">{relativeTime(article.published)}</span>
          <span style={{ marginLeft: "auto", fontSize: 11, color: "var(--label-3)" }}>Esc để đóng</span>
        </div>

        <div className="sheet-body">
          <div className="reader-grid">
            <article>
              <h1 className="reader-title">{article.title}</h1>
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

              {content && (
                <div className="reader-body">
                  {content.blocks.map((block, index) => {
                    switch (block.kind) {
                      case "heading":
                        return <h3 key={index}>{block.text}</h3>;
                      case "quote":
                        return <blockquote key={index}>{block.text}</blockquote>;
                      case "image":
                        return (
                          <figure key={index}>
                            <img src={block.src} alt="" loading="lazy" />
                          </figure>
                        );
                      default:
                        return <p key={index}>{block.text}</p>;
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

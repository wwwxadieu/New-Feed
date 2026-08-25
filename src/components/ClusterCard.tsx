import type { Cluster } from "../lib/types";
import { TOPIC_LABEL } from "../lib/types";
import { hoursSince, relativeTime } from "../lib/format";
import { SourceLogo } from "./SourceLogo";

const TOPIC_TINT: Record<string, string> = {
  ai: "var(--indigo)",
  security: "var(--red)",
  hardware: "var(--teal)",
  device: "var(--blue)",
  startup: "var(--green)",
  ev: "var(--orange)",
  social: "var(--pink)",
  space: "var(--blue)",
  other: "var(--label-3)",
};

interface Props {
  cluster: Cluster;
  index: number;
  lead: boolean;
  onOpen: (cluster: Cluster) => void;
}

export function ClusterCard({ cluster, index, lead, onOpen }: Props) {
  const image = cluster.articles.find((a) => a.image)?.image ?? null;
  // Một báo có thể có nhiều bài trong cùng cụm, nhưng chỉ nên hiện một lần.
  const uniqueSources = [...new Map(cluster.articles.map((a) => [a.sourceId, a.sourceTitle])).entries()];
  const names = uniqueSources.slice(0, 3).map(([, title]) => title);
  const extra = uniqueSources.length - names.length;
  const rising = cluster.sourceCount >= 4 && hoursSince(cluster.newest) < 6;

  return (
    <button
      className={`cluster-card${lead ? " lead" : ""}`}
      style={{ "--i": index } as React.CSSProperties}
      onClick={() => onOpen(cluster)}
    >
      <span className="thumb" style={{ "--tint": TOPIC_TINT[cluster.topic] ?? "var(--blue)" } as React.CSSProperties}>
        <span className="fallback" />
        <span className="glyph">
          <b>{cluster.sourceCount}</b>
          <small>nguồn</small>
        </span>
        {image && (
          <img
            src={image}
            alt=""
            loading="lazy"
            onError={(event) => {
              event.currentTarget.hidden = true;
            }}
          />
        )}
      </span>

      <span>
        <span className="c-meta">
          <span className="pill">{TOPIC_LABEL[cluster.topic] ?? "Khác"}</span>
          <span className="stamp">{relativeTime(cluster.newest)}</span>
          {rising && <span className="pill hot">Đang tăng</span>}
        </span>

        <h2 className="c-title">{cluster.title}</h2>
        {cluster.summary && <p className="c-summary">{cluster.summary}</p>}

        <span className="c-foot">
          <span className="avatar-stack">
            {uniqueSources.slice(0, 4).map(([id, name]) => (
              <SourceLogo key={id} sourceId={id} name={name} />
            ))}
          </span>
          <span className="source-line">
            <b>{cluster.sourceCount}</b> nguồn · {names.join(", ")}
            {extra > 0 ? ` +${extra}` : ""}
          </span>
        </span>
      </span>
    </button>
  );
}

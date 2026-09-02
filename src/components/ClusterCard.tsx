import type { Cluster } from "../lib/types";
import { TOPIC_LABEL } from "../lib/types";
import { hoursSince, relativeTime } from "../lib/format";
import { assetUrl } from "../lib/api";
import { SourceLogo } from "./SourceLogo";
import { TopicIcon } from "./TopicIcons";

const TOPIC_TINT: Record<string, string> = {
  ai: "var(--indigo)",
  security: "var(--red)",
  hardware: "var(--teal)",
  device: "var(--blue)",
  ev: "var(--orange)",
  games: "var(--purple)",
  social: "var(--pink)",
  space: "var(--blue)",
  other: "var(--label-3)",
};

interface Props {
  cluster: Cluster;
  index: number;
  onOpen: (cluster: Cluster) => void;
}

export function ClusterCard({ cluster, index, onOpen }: Props) {
  // Ưu tiên ảnh đã tải sẵn trên máy; chưa có thì mới lấy từ máy chủ của báo.
  const cached = cluster.articles.find((a) => a.thumb)?.thumb ?? null;
  const remote = cluster.articles.find((a) => a.image)?.image ?? null;
  const image = cached ? assetUrl(cached) : remote;
  // Một báo có thể có nhiều bài trong cùng cụm, nhưng chỉ nên hiện một lần.
  const uniqueSources = [...new Map(cluster.articles.map((a) => [a.sourceId, a.sourceTitle])).entries()];
  const names = uniqueSources.slice(0, 3).map(([, title]) => title);
  const extra = uniqueSources.length - names.length;
  const rising = cluster.sourceCount >= 4 && hoursSince(cluster.newest) < 6;
  const translated = cluster.titleVi?.trim();
  const summary = (translated && cluster.summaryVi?.trim()) || cluster.summary;

  return (
    <button
      className="cluster-card poster"
      // Chỉ so le vài thẻ đầu. Không chặn thì thẻ thứ 60 phải đợi hơn một
      // giây rưỡi mới hiện, trông như ứng dụng đang treo chứ không phải hiệu ứng.
      style={{ "--i": Math.min(index, 10) } as React.CSSProperties}
      onClick={() => onOpen(cluster)}
    >
      <span
        className="card-pic"
        style={{ "--tint": TOPIC_TINT[cluster.topic] ?? "var(--blue)" } as React.CSSProperties}
      >
        <span className="fallback" />
        <span className="glyph">
          <TopicIcon topic={cluster.topic} />
        </span>
        {image && (
          <>
            <img
              src={image}
              alt=""
              loading="lazy"
              decoding="async"
              onError={(event) => {
                event.currentTarget.hidden = true;
              }}
            />
            {/* Bản sao đã làm mờ sẵn cho vùng đáy, nơi tấm kính nằm lên. */}
            <img className="blur" src={image} alt="" aria-hidden="true" loading="lazy" decoding="async" />
          </>
        )}
      </span>

      <span className="card-body">
        <span className="c-meta">
          <span className="pill">{TOPIC_LABEL[cluster.topic] ?? "Khác"}</span>
          <span className="stamp">{relativeTime(cluster.newest)}</span>
          {rising && <span className="pill hot">Đang tăng</span>}
          {translated && (
            <span className="pill quiet" title={`Nguyên văn: ${cluster.title}`}>
              đã dịch
            </span>
          )}
        </span>

        <h2 className="c-title">{translated || cluster.title}</h2>
        {summary && <p className="c-summary">{summary}</p>}

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

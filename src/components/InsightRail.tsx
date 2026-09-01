import { useMemo } from "react";
import type { Cluster, Snapshot } from "../lib/types";
import { TOPIC_LABEL } from "../lib/types";
import { clockTime, formatNumber, relativeTime } from "../lib/format";
import { FlameIcon } from "./Icons";

interface Props {
  snapshot: Snapshot;
  clusters: Cluster[];
  onOpen: (cluster: Cluster) => void;
}

function Sparkline({ data }: { data: number[] }) {
  const path = useMemo(() => {
    if (data.length < 2) return null;
    const width = 280;
    const height = 54;
    const pad = 3;
    const max = Math.max(...data);
    const min = Math.min(...data);
    const span = Math.max(max - min, 1);
    const x = (i: number) => pad + (i * (width - pad * 2)) / (data.length - 1);
    const y = (v: number) => height - pad - ((v - min) / span) * (height - pad * 2 - 3);
    const line = data.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
    return {
      width,
      height,
      line,
      area: `${line} L${x(data.length - 1).toFixed(1)} ${height} L${pad} ${height} Z`,
      last: { x: x(data.length - 1), y: y(data[data.length - 1]) },
    };
  }, [data]);

  if (!path) return null;

  return (
    <svg
      className="spark"
      viewBox={`0 0 ${path.width} ${path.height}`}
      preserveAspectRatio="none"
      role="img"
      aria-label="Số bài nhận được theo từng giờ trong 24 giờ qua"
    >
      <defs>
        <linearGradient id="spark-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--blue)" stopOpacity="0.28" />
          <stop offset="100%" stopColor="var(--blue)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={path.area} fill="url(#spark-fill)" />
      <path
        d={path.line}
        fill="none"
        stroke="var(--blue)"
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
      />
      <circle cx={path.last.x} cy={path.last.y} r={3} fill="var(--blue)" />
    </svg>
  );
}

export function InsightRail({ snapshot, clusters, onOpen }: Props) {
  const hot = clusters.slice(0, 5);
  const peak = hot[0]?.score ?? 1;
  const topics = snapshot.topicCounts.slice(0, 7);
  const topicPeak = topics[0]?.[1] ?? 1;

  const withErrors = snapshot.sources.filter((s) => s.lastError).length;
  const active = snapshot.sources.filter((s) => s.enabled).length;

  return (
    <aside className="rail">
      <section className="panel accent" style={{ animationDelay: "0ms" }}>
        <div className="panel-head">
          <span className="panel-icon">
            <FlameIcon />
          </span>
          <h3>Tin hot</h3>
          <span className="note">{clockTime(snapshot.lastRefresh)}</span>
        </div>
        {hot.length === 0 ? (
          <p style={{ margin: 0, fontSize: 12.5, color: "var(--label-3)" }}>Chưa có dữ liệu.</p>
        ) : (
          hot.map((cluster, index) => (
            <button
              className="hot-row"
              key={cluster.id}
              onClick={() => onOpen(cluster)}
              title="Mở tin này"
            >
              <span className="rank">{String(index + 1).padStart(2, "0")}</span>
              <span>
                <p>{cluster.titleVi?.trim() || cluster.title}</p>
                <span className="meter">
                  <i style={{ width: `${Math.round((cluster.score / peak) * 100)}%` }} />
                </span>
                <span className="sub">
                  {cluster.sourceCount} nguồn · {relativeTime(cluster.newest)}
                </span>
              </span>
            </button>
          ))
        )}
      </section>

      <section className="panel" style={{ animationDelay: "35ms" }}>
        <div className="panel-head">
          <h3>Mật độ theo chủ đề</h3>
          <span className="note">số bài</span>
        </div>
        {topics.length === 0 ? (
          <p style={{ margin: 0, fontSize: 12.5, color: "var(--label-3)" }}>Chưa có dữ liệu.</p>
        ) : (
          topics.map(([topic, count]) => (
            <div className="bar-row" key={topic}>
              <span className="name">{TOPIC_LABEL[topic] ?? "Khác"}</span>
              <span className="track">
                <i style={{ width: `${Math.max(Math.round((count / topicPeak) * 100), 4)}%` }} />
              </span>
              <span className="value">{count}</span>
            </div>
          ))
        )}
      </section>

      <section className="panel" style={{ animationDelay: "70ms" }}>
        <div className="panel-head">
          <h3>Tin tức cập nhật theo giờ</h3>
          <span className="note">24 giờ</span>
        </div>
        <Sparkline data={snapshot.hourly} />
        <div className="spark-axis">
          <span>24 giờ trước</span>
          <span>bây giờ</span>
        </div>
      </section>

      <section className="panel" style={{ animationDelay: "105ms" }}>
        <div className="panel-head">
          <h3>Tin tức được lưu</h3>
        </div>
        <div className="stat-grid">
          <div className="stat">
            <span className="n">{formatNumber(snapshot.articleCount)}</span>
            <span className="k">bài đang lưu trên máy</span>
          </div>
          <div className="stat green">
            <span className="n">{active}</span>
            <span className="k">nguồn đang bật</span>
          </div>
          <div className={`stat${withErrors > 0 ? " orange" : ""}`}>
            <span className="n">{withErrors}</span>
            <span className="k">nguồn lỗi</span>
          </div>
        </div>
      </section>
    </aside>
  );
}

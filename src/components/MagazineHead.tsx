import type { Cluster } from "../lib/types";
import { TOPIC_LABEL } from "../lib/types";
import { hoursSince, relativeTime } from "../lib/format";
import { assetUrl } from "../lib/api";
import { SourceLogo } from "./SourceLogo";
import { TopicIcon } from "./TopicIcons";

/** Số tin đặc tả xếp dưới tin hero. */
export const FEATURE_COUNT = 3;
/** Dưới mức này thì không đủ tin để dựng phân cấp, dùng lưới thường. */
export const MAGAZINE_MIN = FEATURE_COUNT + 2;

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

/** Ảnh tốt nhất đang có: bản lớn nếu đã tải, không thì bản lưới, cuối cùng
 *  mới tới địa chỉ trên máy chủ của báo. */
function pickImage(cluster: Cluster): string | null {
  const hero = cluster.articles.find((a) => a.hero)?.hero;
  if (hero) return assetUrl(hero);
  const cached = cluster.articles.find((a) => a.thumb)?.thumb;
  if (cached) return assetUrl(cached);
  return cluster.articles.find((a) => a.image)?.image ?? null;
}

function sourcesOf(cluster: Cluster) {
  return [...new Map(cluster.articles.map((a) => [a.sourceId, a.sourceTitle])).entries()];
}

function Meta({ cluster }: { cluster: Cluster }) {
  const rising = cluster.sourceCount >= 4 && hoursSince(cluster.newest) < 6;
  const translated = cluster.titleVi?.trim();
  return (
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
  );
}

function Foot({ cluster }: { cluster: Cluster }) {
  const unique = sourcesOf(cluster);
  const names = unique.slice(0, 3).map(([, title]) => title);
  const extra = unique.length - names.length;
  return (
    <span className="c-foot">
      <span className="avatar-stack">
        {unique.slice(0, 4).map(([id, name]) => (
          <SourceLogo key={id} sourceId={id} name={name} />
        ))}
      </span>
      <span className="source-line">
        <b>{cluster.sourceCount}</b> nguồn · {names.join(", ")}
        {extra > 0 ? ` +${extra}` : ""}
      </span>
    </span>
  );
}

function Picture({ cluster, className }: { cluster: Cluster; className: string }) {
  const image = pickImage(cluster);
  return (
    <span
      className={className}
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
            decoding="async"
            onError={(event) => {
              event.currentTarget.hidden = true;
            }}
          />
          {/* Hai lớp mờ chồng nhau, bán kính khác nhau, mỗi lớp một dải mặt
              nạ riêng: càng xuống đáy càng nhoè. Một lớp duy nhất thì chỉ là
              một độ nhoè cố định mờ dần độ đục, không phải nhoè tăng dần. */}
          <img className="blur soft" src={image} alt="" aria-hidden="true" decoding="async" />
          <img className="blur deep" src={image} alt="" aria-hidden="true" decoding="async" />
        </>
      )}
    </span>
  );
}

interface Props {
  hero: Cluster;
  features: Cluster[];
  onOpen: (cluster: Cluster) => void;
}

/**
 * Phần đầu dòng tin dựng theo lối tạp chí: một tin hero rồi một hàng tin
 * đặc tả, phía dưới mới là lưới thẻ đều của dashboard.
 *
 * Chỉ áp cho vài cụm đầu là có chủ ý. Phân cấp kiểu tạp chí cần một tín
 * hiệu đủ mạnh để nói tin nào xứng đáng lớn hơn; ở đây tín hiệu đó là điểm
 * cụm, mà điểm chỉ tách bạch ở vài cụm đầu. Xuống tới cụm thứ hai ba mươi
 * thì điểm gần bằng nhau, lúc đó thẻ to nhỏ khác nhau không còn là phân cấp
 * mà thành lộn xộn.
 */
export function MagazineHead({ hero, features, onOpen }: Props) {
  const heroTitle = hero.titleVi?.trim() || hero.title;
  const heroSummary = (hero.titleVi?.trim() && hero.summaryVi?.trim()) || hero.summary;

  return (
    <div className="magazine">
      <button className="hero-card poster" onClick={() => onOpen(hero)}>
        <Picture cluster={hero} className="card-pic" />
        <span className="card-body">
          <Meta cluster={hero} />
          <h2 className="hero-title">{heroTitle}</h2>
          {heroSummary && <p className="hero-summary">{heroSummary}</p>}
          <Foot cluster={hero} />
        </span>
      </button>

      <div className="feature-row">
        {features.map((cluster, index) => {
          const title = cluster.titleVi?.trim() || cluster.title;
          return (
            <button
              key={cluster.id}
              className="feature-card poster"
              onClick={() => onOpen(cluster)}
              style={{ "--i": index } as React.CSSProperties}
            >
              <Picture cluster={cluster} className="card-pic" />
              <span className="card-body">
                <Meta cluster={cluster} />
                <h3 className="feature-title">{title}</h3>
                <Foot cluster={cluster} />
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

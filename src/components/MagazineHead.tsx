import { useEffect, useState } from "react";
import type { Cluster } from "../lib/types";
import { TOPIC_LABEL } from "../lib/types";
import { hoursSince, relativeTime } from "../lib/format";
import { assetUrl } from "../lib/api";
import { SourceLogo } from "./SourceLogo";
import { TopicIcon } from "./TopicIcons";

/** Số tin đặc tả xếp dưới tin hero. */
export const FEATURE_COUNT = 3;
/** Số tin thay nhau chạy ở tấm hero. */
export const HERO_SLIDES = 5;
/**
 * Nhịp đổi tin ở tấm hero.
 *
 * Đủ dài để đọc hết tiêu đề và ba dòng tóm tắt rồi còn kịp quyết định có mở
 * hay không; đủ ngắn để tấm lớn nhất màn hình không đứng im như ảnh dán.
 */
const SLIDE_MS = 7000;
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

/**
 * Tấm hero chạy luân phiên vài tin đầu bảng.
 *
 * Các tấm nằm chồng lên nhau trong cùng một khung và đổi bằng cách mờ chồng,
 * không trượt ngang: tấm này cao 21/9 chiếm gần trọn bề ngang dòng tin, cho
 * nó trượt qua lại là kéo theo cả một dải ảnh lớn di chuyển mỗi bảy giây,
 * đúng thứ làm người đọc mỏi mắt ở những trang tin đặt băng chuyền.
 */
function HeroDeck({ slides, onOpen }: { slides: Cluster[]; onOpen: (c: Cluster) => void }) {
  const [index, setIndex] = useState(0);
  const [held, setHeld] = useState(false);
  // Mốc xa nhất đã tới. Tấm đã dựng thì giữ luôn, không tháo ra: quay vòng
  // mà tháo thì mỗi vòng lại tải và giải mã lại từng ấy ảnh khổ lớn.
  const [reach, setReach] = useState(1);

  useEffect(() => {
    setReach((current) => Math.max(current, index + 1));
  }, [index]);

  // Danh sách có thể ngắn lại sau một lượt làm mới.
  useEffect(() => {
    setIndex((current) => (current < slides.length ? current : 0));
  }, [slides.length]);

  // Hẹn giờ theo từng tấm chứ không phải một nhịp chạy suốt: bấm sang tấm
  // khác thì tấm đó cũng được trọn bảy giây, không bị cắt ngang giữa chừng.
  useEffect(() => {
    if (held || slides.length < 2) return;
    const timer = window.setTimeout(
      () => setIndex((current) => (current + 1) % slides.length),
      SLIDE_MS,
    );
    return () => window.clearTimeout(timer);
  }, [index, held, slides.length]);

  return (
    <div
      className="hero-deck"
      style={{ "--slide-ms": `${SLIDE_MS}ms` } as React.CSSProperties}
      // Dừng khi con trỏ đang ở trên tấm hoặc khi bàn phím vừa nhảy vào:
      // đổi tin ngay lúc người ta đang đọc hoặc sắp bấm là cướp mất thao tác.
      onMouseEnter={() => setHeld(true)}
      onMouseLeave={() => setHeld(false)}
      onFocusCapture={() => setHeld(true)}
      onBlurCapture={() => setHeld(false)}
    >
      {slides.map((cluster, i) => {
        // Lượt vẽ đầu chỉ dựng tấm đang xem và tấm kế tiếp, rồi mở dần theo
        // nhịp chạy. Dựng sẵn cả năm ngay từ đầu là năm tấm ảnh khổ lớn cùng
        // hai lớp nhoè mỗi tấm phải tải và giải mã trước khi thấy tin đầu.
        if (i > reach) return null;
        const active = i === index;
        const title = cluster.titleVi?.trim() || cluster.title;
        const summary = (cluster.titleVi?.trim() && cluster.summaryVi?.trim()) || cluster.summary;
        return (
          <button
            key={cluster.id}
            className={`hero-card poster${active ? " is-active" : ""}`}
            onClick={() => onOpen(cluster)}
            aria-hidden={!active}
            tabIndex={active ? 0 : -1}
          >
            <Picture cluster={cluster} className="card-pic" />
            <span className="card-body">
              <Meta cluster={cluster} />
              <h2 className="hero-title">{title}</h2>
              {summary && <p className="hero-summary">{summary}</p>}
              <Foot cluster={cluster} />
            </span>
          </button>
        );
      })}

      {slides.length > 1 && (
        <>
          {/* Vạch chạy hết bề ngang đúng bằng nhịp đổi tấm, để người đọc biết
              sắp tới lượt đổi chứ không bị đổi bất ngờ. Đổi khoá theo tấm nên
              nó chạy lại từ đầu mỗi lượt. */}
          <i className={`hero-progress${held ? " held" : ""}`} key={index} />
          <div className="hero-dots">
            {slides.map((cluster, i) => (
              <button
                key={cluster.id}
                className={`hero-dot${i === index ? " on" : ""}`}
                aria-label={`Tin ${i + 1} trong ${slides.length}`}
                aria-current={i === index}
                onClick={() => setIndex(i)}
              />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

interface Props {
  heroes: Cluster[];
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
export function MagazineHead({ heroes, features, onOpen }: Props) {
  return (
    <div className="magazine">
      <HeroDeck slides={heroes} onOpen={onOpen} />

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

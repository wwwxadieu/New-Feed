import type { Cluster } from "../lib/types";
import { relativeTime } from "../lib/format";
import { FlameIcon } from "./Icons";

interface Props {
  clusters: Cluster[];
  onOpen: (cluster: Cluster) => void;
}

/** Số tin nóng hiện trên dải. Sáu là đủ để liếc mà chưa thành một danh sách
 *  thứ hai cạnh tranh với dòng tin bên dưới. */
const MAX_HOT = 6;

/**
 * Dải tin đang nóng nằm ngang phía trên dòng tin.
 *
 * Trước đây phần này là một cột dọc bên phải. Cột đó ăn 328px bề ngang suốt
 * chiều cao cửa sổ, tức lấy đúng chỗ của thứ người dùng vào đây để đọc. Nằm
 * ngang thì nó chỉ chiếm một dải cao khoảng 76px, và dòng tin lấy lại được
 * toàn bộ bề ngang.
 */
export function HotStrip({ clusters, onOpen }: Props) {
  const hot = clusters.slice(0, MAX_HOT);
  if (hot.length === 0) return null;

  return (
    <section className="hot-strip" aria-label="Tin đang nóng">
      <span className="hot-strip-label">
        <FlameIcon />
        Đang nóng
      </span>

      <div className="hot-strip-track">
        {hot.map((cluster, index) => {
          const title = cluster.titleVi?.trim() || cluster.title;
          return (
            <button
              key={cluster.id}
              className="hot-tab"
              onClick={() => onOpen(cluster)}
              title={title}
              style={{ "--i": index } as React.CSSProperties}
            >
              <span className="hot-tab-rank">{index + 1}</span>
              <span className="hot-tab-body">
                <span className="hot-tab-title">{title}</span>
                {/* Không lặp lại nhãn "Đang tăng" ở đây: cả dải này vốn đã là
                    tin đang nóng, mà thêm vào thì dòng meta dài quá một dòng và
                    bị cắt giữa chữ. Nhãn đó vẫn còn trên thẻ tin bên dưới. */}
                <span className="hot-tab-meta">
                  {cluster.sourceCount} nguồn · {relativeTime(cluster.newest)}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
}

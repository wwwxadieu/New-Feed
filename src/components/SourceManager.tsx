import { useEffect, useState } from "react";
import type { Source } from "../lib/types";
import { clockTime, formatNumber, hostOf } from "../lib/format";
import * as api from "../lib/api";
import { CloseIcon, PlusIcon, TrashIcon } from "./Icons";
import { SourceLogo } from "./SourceLogo";

interface Props {
  sources: Source[];
  busy: boolean;
  translateEmail: string;
  onTranslateEmail: (email: string) => void;
  weatherPlace: string;
  onWeatherPlace: (place: string) => void;
  onAdd: (input: string) => Promise<void>;
  onRemove: (id: string) => void;
  onToggle: (id: string, enabled: boolean) => void;
  onClose: () => void;
}

export function SourceManager({
  sources,
  busy,
  translateEmail,
  onTranslateEmail,
  weatherPlace,
  onWeatherPlace,
  onAdd,
  onRemove,
  onToggle,
  onClose,
}: Props) {
  const [input, setInput] = useState("");
  const [email, setEmail] = useState(translateEmail);
  const [place, setPlace] = useState(weatherPlace);
  // Nơi thực sự đang lấy số liệu, để người dùng biết thành phố mình gõ có
  // được nhận hay không — gõ sai thì ứng dụng lặng lẽ lùi về tự dò theo IP.
  const [resolved, setResolved] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    api.getWeather().then((w) => { if (alive) setResolved(w?.place ?? null); }).catch(() => {});
    return () => { alive = false; };
  }, [weatherPlace]);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    const value = input.trim();
    if (!value || busy) return;
    await onAdd(value);
    setInput("");
  };

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="sheet compact" role="dialog" aria-modal="true" aria-label="Quản lý nguồn tin">
        <div className="sheet-bar">
          <button className="icon-button" onClick={onClose} aria-label="Đóng">
            <CloseIcon />
          </button>
          <h2>Nguồn tin</h2>
          <span style={{ marginLeft: "auto", fontSize: 11.5, color: "var(--label-3)" }}>
            {sources.length} nguồn
          </span>
        </div>

        <form className="add-form" onSubmit={submit}>
          <input
            className="text-field"
            placeholder="Dán địa chỉ trang tin hoặc RSS…"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            disabled={busy}
            autoFocus
          />
          <button className="link-button" type="submit" disabled={busy}>
            <PlusIcon />
            {busy ? "Đang dò…" : "Thêm"}
          </button>
        </form>

        <p className="form-hint">
          Dán được cả địa chỉ trang chủ, ứng dụng tự dò feed. Ví dụ: vnexpress.net/so-hoa
        </p>

        <div className="sheet-body">
          <div className="source-list">
            {sources.length === 0 ? (
              <p style={{ padding: "28px 14px", textAlign: "center", color: "var(--label-3)", fontSize: 13 }}>
                Chưa có nguồn nào. Thêm nguồn đầu tiên ở ô phía trên.
              </p>
            ) : (
              sources.map((source, index) => (
                <div
                  className="source-row"
                  key={source.id}
                  title={`${source.title} — ${hostOf(source.feedUrl)}`}
                  style={{ "--i": index } as React.CSSProperties}
                >
                  <button
                    className="switch"
                    role="switch"
                    aria-checked={source.enabled}
                    aria-label={`Bật tắt ${source.title}`}
                    onClick={() => onToggle(source.id, !source.enabled)}
                  >
                    <i />
                  </button>

                  <SourceLogo sourceId={source.id} name={source.title} size={30} radius={9} />

                  <div className="info">
                    <b>{source.title}</b>
                    {source.lastError ? (
                      <small className="error" title={source.lastError}>
                        {source.lastError}
                      </small>
                    ) : (
                      /* Bỏ tên miền khỏi dòng này: xếp hai cột thì mỗi cột chỉ
                         còn chỗ cho khoảng 33 ký tự, mà cả tên miền lẫn nhãn
                         ngôn ngữ là 46 — nhãn ngôn ngữ bị cắt mất. Tên miền vốn
                         trùng ý với tên nguồn nên nhường chỗ; vẫn xem được ở
                         chú thích khi rê chuột vào hàng. */
                      <small>
                        {formatNumber(source.articleCount)} bài · {clockTime(source.lastFetched)}
                        {source.language === "other" ? " · tiếng nước ngoài" : ""}
                      </small>
                    )}
                  </div>

                  <button
                    className="danger-button"
                    onClick={() => onRemove(source.id)}
                    aria-label={`Xoá ${source.title}`}
                    title="Xoá nguồn"
                  >
                    <TrashIcon />
                  </button>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Nằm ngoài vùng cuộn, ghim ở đáy tấm. Để bên trong thì thêm nhiều
            nguồn là phải cuộn qua hết danh sách mới tới được cài đặt — đo với
            20 nguồn thì khối này nằm ở vị trí 1120px trong vùng cuộn.
            Hai ô xếp cạnh nhau: xếp dọc thì riêng phần này đã chiếm 289px. */}
        <div className="sheet-settings">
          <div className="setting">
            <span className="section-title">Hạn mức dịch</span>
            <div className="setting-row">
              <input
                className="text-field"
                type="email"
                placeholder="email@example.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
              />
              <button
                className="link-button"
                onClick={() => onTranslateEmail(email.trim())}
                disabled={email.trim() === translateEmail}
              >
                Lưu
              </button>
            </div>
            <p className="setting-hint">
              Ẩn danh được 5.000 ký tự mỗi ngày, khai email thì 50.000.
            </p>
          </div>

          <div className="setting">
            <span className="section-title">Vị trí thời tiết</span>
            <div className="setting-row">
              <input
                className="text-field"
                type="text"
                placeholder="Hà Nội, Đà Nẵng…"
                value={place}
                onChange={(e) => setPlace(e.target.value)}
              />
              <button
                className="link-button"
                onClick={() => onWeatherPlace(place.trim())}
                disabled={place.trim() === weatherPlace}
              >
                Lưu
              </button>
            </div>
            <p className="setting-hint">
              {resolved ? (
                <>
                  Đang lấy ở <b>{resolved}</b>. Để trống là tự dò theo IP, sai nếu dùng VPN.
                </>
              ) : (
                <>Để trống là tự dò theo IP, sai nếu dùng VPN.</>
              )}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

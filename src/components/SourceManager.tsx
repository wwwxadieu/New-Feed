import { useState } from "react";
import type { Source } from "../lib/types";
import { clockTime, formatNumber, hostOf } from "../lib/format";
import { CloseIcon, PlusIcon, TrashIcon } from "./Icons";
import { SourceLogo } from "./SourceLogo";

interface Props {
  sources: Source[];
  busy: boolean;
  translateEmail: string;
  onTranslateEmail: (email: string) => void;
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
  onAdd,
  onRemove,
  onToggle,
  onClose,
}: Props) {
  const [input, setInput] = useState("");
  const [email, setEmail] = useState(translateEmail);

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
          Có thể dán địa chỉ trang chủ — ứng dụng sẽ tự dò feed. Ví dụ: vnexpress.net/so-hoa hoặc
          arstechnica.com
        </p>

        <div className="sheet-body">
          {sources.length === 0 ? (
            <p style={{ padding: "28px 14px", textAlign: "center", color: "var(--label-3)", fontSize: 13 }}>
              Chưa có nguồn nào. Thêm nguồn đầu tiên ở ô phía trên.
            </p>
          ) : (
            sources.map((source, index) => (
              <div className="source-row" key={source.id} style={{ "--i": index } as React.CSSProperties}>
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
                    <small>
                      {hostOf(source.feedUrl)} · {formatNumber(source.articleCount)} bài ·{" "}
                      {clockTime(source.lastFetched)}
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

          <div className="sheet-section">
            <span className="section-title">Hạn mức dịch</span>
            <p className="form-hint" style={{ padding: "0 0 8px", border: "none" }}>
              Dịch vụ dịch cho 5.000 ký tự mỗi ngày khi dùng ẩn danh, hoặc 50.000 nếu khai báo
              một địa chỉ email. Để trống vẫn dùng được bình thường.
            </p>
            <div style={{ display: "flex", gap: 8 }}>
              <input
                className="text-field"
                type="email"
                placeholder="email@example.com (không bắt buộc)"
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
          </div>
        </div>
      </div>
    </div>
  );
}

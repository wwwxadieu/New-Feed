import { isDesktop } from "../lib/api";
import { CloseIcon, MaximizeIcon, MinimizeIcon } from "./Icons";

async function windowAction(action: "minimize" | "toggleMaximize" | "close") {
  if (!isDesktop) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const current = getCurrentWindow();
  if (action === "minimize") await current.minimize();
  else if (action === "toggleMaximize") await current.toggleMaximize();
  else await current.close();
}

/**
 * Thanh tiêu đề tự vẽ để giữ đường nét liền mạch với phần kính bên dưới.
 * Nút điều khiển đặt bên phải theo thói quen của Windows.
 */
export function TitleBar() {
  return (
    <div className="titlebar">
      <span className="mark" />
      <span className="name">News Feed</span>
      <div className="drag" data-tauri-drag-region onDoubleClick={() => windowAction("toggleMaximize")} />
      <div className="win-controls">
        <button onClick={() => windowAction("minimize")} aria-label="Thu nhỏ">
          <MinimizeIcon />
        </button>
        <button onClick={() => windowAction("toggleMaximize")} aria-label="Phóng to">
          <MaximizeIcon />
        </button>
        <button className="close" onClick={() => windowAction("close")} aria-label="Đóng">
          <CloseIcon size={14} />
        </button>
      </div>
    </div>
  );
}

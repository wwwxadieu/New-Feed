import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/inter/index.css";
// Bộ nghiêng thật, có sẵn dấu tiếng Việt. Không nạp thì trình duyệt tự bóp
// nghiêng chữ đứng và dấu bị méo.
import "@fontsource-variable/inter/wght-italic.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/app.css";
import App from "./App";
import { suppressWebviewContextMenu } from "./lib/nativeChrome";

// Bản dev giữ nguyên menu chuột phải, vì "Inspect" trong đó là đường mở
// devtools. Bản dựng để dùng thật thì không cần tới nó nữa.
if (import.meta.env.PROD) {
  suppressWebviewContextMenu();
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

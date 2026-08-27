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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

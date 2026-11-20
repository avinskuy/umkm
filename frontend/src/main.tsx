import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./app/App";
import "./styles/base/reset.css";
import "./styles/base/global.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
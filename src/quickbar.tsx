import React from "react";
import ReactDOM from "react-dom/client";
import QuickBar from "./windows/QuickBar";
import "./styles.css";
import { markPlatform } from "./lib/platform";

markPlatform();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QuickBar />
  </React.StrictMode>,
);

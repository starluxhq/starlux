import React from "react";
import ReactDOM from "react-dom/client";
import Workspace from "./windows/Workspace";
import "./styles.css";
import { markPlatform } from "./lib/platform";

markPlatform();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Workspace />
  </React.StrictMode>,
);

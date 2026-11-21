import React from "react";
import ReactDOM from "react-dom/client";

import { App } from "./App.tsx";
import "./styles/app.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Не найден корневой элемент #root");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

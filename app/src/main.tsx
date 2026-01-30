// Path: app/src/main.tsx
// Description: React entry point - mounts App with ConfigProvider and AgentProvider

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app.js";
import { ConfigProvider } from "./hooks/use_config.js";
import { AgentProvider } from "./hooks/use_agent.js";
import "./styles/main.css";
import "./styles/bundle_column.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Root element not found");
}

createRoot(container).render(
  <StrictMode>
    <ConfigProvider>
      <AgentProvider>
        <App />
      </AgentProvider>
    </ConfigProvider>
  </StrictMode>
);

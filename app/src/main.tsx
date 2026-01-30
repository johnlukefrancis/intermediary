// Path: app/src/main.tsx
// Description: React entry point - mounts App with ConfigProvider and AgentProvider

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app.js";
import { ConfigProvider } from "./hooks/use_config.js";
import { AgentProvider } from "./hooks/use_agent.js";
// CSS imports - ORDER MATTERS (tokens -> theme -> accents -> effects -> layout -> panels -> chrome -> components)
import "./styles/tokens.css";
import "./styles/theme_dark.css";
import "./styles/theme_accents.css";
import "./styles/effects.css";
import "./styles/motion.css";
import "./styles/a11y.css";
import "./styles/badges.css";
import "./styles/main.css";
import "./styles/panels.css";
import "./styles/scrollbars.css";
import "./styles/chrome.css";
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

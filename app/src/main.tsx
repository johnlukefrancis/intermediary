// Path: app/src/main.tsx
// Description: React entry point - mounts App with AgentProvider to DOM

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app.js";
import { AgentProvider } from "./hooks/use_agent.js";
import "./styles/main.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("Root element not found");
}

createRoot(container).render(
  <StrictMode>
    <AgentProvider>
      <App />
    </AgentProvider>
  </StrictMode>
);

// Path: app/src/main.tsx
// Description: React entry point - mounts App with ConfigProvider and AgentProvider

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app.js";
import { ConfigProvider } from "./hooks/use_config.js";
import { AgentProvider } from "./hooks/use_agent.js";
// CSS imports - ORDER MATTERS (tokens -> theme -> accents -> effects -> motion -> boot -> a11y -> layout -> panels -> chrome -> components)
import "./styles/tokens.css";
import "./styles/theme_dark.css";
import "./styles/theme_warm.css";
import "./styles/theme_light.css";
import "./styles/theme_accents.css";
import "./styles/effects.css";
import "./styles/motion.css";
import "./styles/boot.css";
import "./styles/a11y.css";
import "./styles/badges.css";
import "./styles/main.css";
import "./styles/panels.css";
import "./styles/auto_files.css";
import "./styles/auto_files_controls.css";
import "./styles/auto_files_telemetry.css";
import "./styles/auto_files_responsive.css";
import "./styles/scrollbars.css";
import "./styles/chrome.css";
// xterm's base sheet precedes the component sheets so terminal_column.css can override its insets
import "@xterm/xterm/css/xterm.css";
import "./styles/bundle_column.css";
import "./styles/bundle_file_explorer_drop.css";
import "./styles/bundle_file_explorer_selection.css";
import "./styles/text_workspace.css";
import "./styles/text_workspace_semantics.css";
import "./styles/source_control.css";
import "./styles/source_control_sections.css";
import "./styles/source_control_rows.css";
import "./styles/diff_workspace.css";
import "./styles/image_diff_workspace.css";
// Stream sheets land last so the panel can override the shared panel and diff grammar
import "./styles/stream/stream_panel.css";
import "./styles/stream/stream_card.css";
import "./styles/stream/stream_card_body.css";
import "./styles/stream/stream_card_image.css";
// The choreography sheet lands after motion.css and every stream sheet: it carves the
// stream scroller out of the universal governor pause and overrides the deleted-line strike.
import "./styles/stream/stream_motion.css";

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

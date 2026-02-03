// Path: app/src/components/status_bar.tsx
// Description: Status bar with connection status LED, error display, and options button

import { useState } from "react";
import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import { useConfig } from "../hooks/use_config.js";
import type { ConnectionStatus } from "../lib/agent/connection_state.js";
import { OptionsOverlay } from "./options_overlay.js";
import "../styles/status_bar.css";

interface ConnectionDisplay {
  cssClass: string;
  label: string;
  showPulse: boolean;
}

function getConnectionDisplay(status: ConnectionStatus, attempts: number): ConnectionDisplay {
  switch (status) {
    case "connected":
      return { cssClass: "connected", label: "Connected", showPulse: false };
    case "connecting":
      return { cssClass: "connecting", label: "Connecting...", showPulse: true };
    case "reconnecting":
      return { cssClass: "reconnecting", label: `Reconnecting (${attempts})`, showPulse: true };
    case "disconnected":
      return { cssClass: "disconnected", label: "Offline", showPulse: false };
  }
}

export function StatusBar(): React.JSX.Element {
  const {
    autoStageOnChange,
    setAutoStageOnChange,
    connectionState,
    appPaths,
    helloState,
    agentError,
  } = useAgent();
  const {
    config,
    setGlobalExcludes,
    setOutputWindowsRoot,
    setTabThemeAccent,
    setTabThemeTexture,
    clearTabTheme,
    setRecentFilesLimit,
  } = useConfig();
  const [optionsOpen, setOptionsOpen] = useState(false);

  const { status, reconnectAttempts, lastError: connectionError } = connectionState;
  const display = getConnectionDisplay(status, reconnectAttempts);

  const agentErrorMessage = agentError
    ? `${agentError.message}${
        agentError.details?.docPath ? ` (See ${agentError.details.docPath})` : ""
      }`
    : null;

  // Show connection error when not connected, agent/hello error when connected
  const errorToShow =
    status === "connected" ? agentErrorMessage ?? helloState.lastError : connectionError;

  return (
    <>
      <div className="status-bar">
        <div className="status-left">
          <span className={`status-connection ${display.cssClass}${display.showPulse ? " pulsing" : ""}`}>
            <span className="led-dot" aria-hidden="true" />
            <span className="led-label">{display.label}</span>
          </span>
          {errorToShow && (
            <>
              <span className="chrome-sep" aria-hidden="true">·</span>
              <span className="status-error" title={errorToShow}>
                {errorToShow.length > 50 ? `${errorToShow.slice(0, 50)}...` : errorToShow}
              </span>
            </>
          )}
        </div>

        <div className="status-right">
          <button
            type="button"
            className="options-button"
            onClick={() => {
              setOptionsOpen(true);
            }}
          >
            Options
          </button>
        </div>
      </div>

      {optionsOpen && (
        <OptionsOverlay
          autoStageOnChange={autoStageOnChange}
          setAutoStageOnChange={setAutoStageOnChange}
          appPaths={appPaths}
          globalExcludes={config.globalExcludes}
          setGlobalExcludes={setGlobalExcludes}
          setOutputWindowsRoot={setOutputWindowsRoot}
          recentFilesLimit={config.recentFilesLimit}
          setRecentFilesLimit={setRecentFilesLimit}
          repos={config.repos}
          tabThemes={config.tabThemes}
          setTabThemeAccent={setTabThemeAccent}
          setTabThemeTexture={setTabThemeTexture}
          clearTabTheme={clearTabTheme}
          onClose={() => {
            setOptionsOpen(false);
          }}
        />
      )}
    </>
  );
}

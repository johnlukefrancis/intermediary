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
    agentDiagnostics,
    platformSupportsWsl,
    restartAgent,
  } = useAgent();
  const {
    config,
    setGlobalExcludes,
    setClassificationExcludes,
    setOutputWindowsRoot,
    setAgentAutoStart,
    setAgentDistro,
    setTabThemeAccent,
    setTabThemeTexture,
    clearTabTheme,
    setRecentFilesLimit,
    setThemeMode,
    setUiMode,
    loadError,
    saveError,
    resetConfig,
    renameRepoLabel,
    renameGroupLabel,
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
    agentDiagnostics
      ? null
      : status === "connected"
        ? agentErrorMessage ?? helloState.lastError
        : connectionError;
  const configErrorMessage = loadError
    ? `Config load failed: ${loadError}`
    : saveError
      ? `Config save failed: ${saveError}`
      : null;
  const errorItems = [errorToShow, configErrorMessage].filter(
    (item): item is string => Boolean(item)
  );

  return (
    <>
      <div className="status-bar">
        <div className="status-left">
          <span className={`status-connection ${display.cssClass}${display.showPulse ? " pulsing" : ""}`}>
            <span className="led-dot" aria-hidden="true" />
            <span className="led-label">{display.label}</span>
          </span>
          {errorItems.map((item, index) => (
            <span key={`${index}-${item}`}>
              <span className="chrome-sep" aria-hidden="true">·</span>
              <span className="status-error" title={item}>
                {item.length > 50 ? `${item.slice(0, 50)}...` : item}
              </span>
            </span>
          ))}
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
          agentAutoStart={config.agentAutoStart}
          setAgentAutoStart={setAgentAutoStart}
          supportsWsl={platformSupportsWsl}
          agentDistro={config.agentDistro}
          setAgentDistro={setAgentDistro}
          restartAgent={restartAgent}
          appPaths={appPaths}
          globalExcludes={config.globalExcludes}
          setGlobalExcludes={setGlobalExcludes}
          classificationExcludes={config.classificationExcludes}
          setClassificationExcludes={setClassificationExcludes}
          setOutputWindowsRoot={setOutputWindowsRoot}
          recentFilesLimit={config.recentFilesLimit}
          setRecentFilesLimit={setRecentFilesLimit}
          repos={config.repos}
          tabThemes={config.tabThemes}
          themeMode={config.themeMode}
          setThemeMode={setThemeMode}
          uiMode={config.uiMode}
          setUiMode={setUiMode}
          setTabThemeAccent={setTabThemeAccent}
          setTabThemeTexture={setTabThemeTexture}
          clearTabTheme={clearTabTheme}
          renameRepoLabel={renameRepoLabel}
          renameGroupLabel={renameGroupLabel}
          resetConfig={resetConfig}
          onClose={() => {
            setOptionsOpen(false);
          }}
        />
      )}
    </>
  );
}

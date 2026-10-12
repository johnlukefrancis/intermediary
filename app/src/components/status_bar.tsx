// Path: app/src/components/status_bar.tsx
// Description: Status strip: connection LED and error ticker, system-load meters, and the options button

import { useState } from "react";
import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import { useConfig } from "../hooks/use_config.js";
import { OptionsOverlay } from "./options_overlay.js";
import { StatusConnection } from "./status_connection.js";
import { SystemLoadCluster } from "./system_load/system_load_cluster.js";
import "../styles/status_bar.css";

export function StatusBar({ documentHidden }: { documentHidden: boolean }): React.JSX.Element {
  const {
    autoStageOnChange,
    setAutoStageOnChange,
    appPaths,
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
    setWindowOpacityPercent,
    setTextureIntensityPercent,
    setCrosstalkBubblesEnabled,
    resetConfig,
    renameRepoLabel,
    renameGroupLabel,
  } = useConfig();
  const [optionsOpen, setOptionsOpen] = useState(false);

  return (
    <>
      <div className="status-bar">
        <StatusConnection />

        <div className="status-right">
          <SystemLoadCluster documentHidden={documentHidden} />
          <button
            type="button"
            className={`options-button${optionsOpen ? " options-button--active" : ""}`}
            onClick={() => {
              setOptionsOpen((previous) => !previous);
            }}
            aria-haspopup="dialog"
            aria-expanded={optionsOpen}
            aria-controls="options-overlay-panel"
            aria-label={optionsOpen ? "Close options" : "Open options"}
          >
            {optionsOpen ? "CLOSE [x]" : "OPTIONS"}
          </button>
        </div>
      </div>

      {optionsOpen && (
        <OptionsOverlay
          autoStageOnChange={autoStageOnChange}
          setAutoStageOnChange={setAutoStageOnChange}
          crosstalkBubblesEnabled={config.crosstalkBubblesEnabled}
          setCrosstalkBubblesEnabled={setCrosstalkBubblesEnabled}
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
          windowOpacityPercent={config.windowOpacityPercent}
          setWindowOpacityPercent={setWindowOpacityPercent}
          textureIntensityPercent={config.textureIntensityPercent}
          setTextureIntensityPercent={setTextureIntensityPercent}
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

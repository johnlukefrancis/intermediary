// Path: app/src/components/status_bar.tsx
// Description: Status bar with connection status LED, error display, and options button

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
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

const ERROR_ROTATION_INTERVAL_MS = 5000;
const ERROR_MARQUEE_GAP_PX = 32;

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
  const [activeErrorIndex, setActiveErrorIndex] = useState(0);
  const [isTickerOverflowing, setIsTickerOverflowing] = useState(false);
  const [tickerOverflowDistance, setTickerOverflowDistance] = useState(0);
  const tickerViewportRef = useRef<HTMLSpanElement>(null);
  const tickerSegmentRef = useRef<HTMLSpanElement>(null);

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
  const errorSignature = errorItems.join("\u0001");

  useEffect(() => {
    setActiveErrorIndex(0);
  }, [errorSignature]);

  useEffect(() => {
    if (errorItems.length <= 1) {
      return;
    }

    const timer = window.setInterval(() => {
      setActiveErrorIndex((previous) => {
        if (errorItems.length === 0) {
          return 0;
        }
        return (previous + 1) % errorItems.length;
      });
    }, ERROR_ROTATION_INTERVAL_MS);

    return () => {
      window.clearInterval(timer);
    };
  }, [errorItems.length]);

  const activeErrorMessage = useMemo(() => {
    if (errorItems.length === 0) {
      return null;
    }
    return errorItems[activeErrorIndex % errorItems.length] ?? errorItems[0] ?? null;
  }, [activeErrorIndex, errorItems]);

  const measureTickerOverflow = useCallback(() => {
    const viewport = tickerViewportRef.current;
    const segment = tickerSegmentRef.current;
    if (!viewport || !segment || !activeErrorMessage) {
      setIsTickerOverflowing(false);
      setTickerOverflowDistance(0);
      return;
    }

    const segmentWidth = segment.scrollWidth;
    const viewportWidth = viewport.clientWidth;
    if (segmentWidth <= viewportWidth) {
      setIsTickerOverflowing(false);
      setTickerOverflowDistance(0);
      return;
    }

    setIsTickerOverflowing(true);
    setTickerOverflowDistance(segmentWidth + ERROR_MARQUEE_GAP_PX);
  }, [activeErrorMessage]);

  useLayoutEffect(() => {
    measureTickerOverflow();
  }, [measureTickerOverflow]);

  useEffect(() => {
    if (!activeErrorMessage) {
      return;
    }

    const handleResize = (): void => {
      measureTickerOverflow();
    };

    window.addEventListener("resize", handleResize);
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, [activeErrorMessage, measureTickerOverflow]);

  const tickerTrackStyle = useMemo(
    () => {
      const tickerDurationMs = Math.max(6000, Math.round(tickerOverflowDistance * 28));
      return {
        "--status-error-scroll-distance": `${tickerOverflowDistance}px`,
        "--status-error-scroll-duration": `${tickerDurationMs}ms`,
        "--status-error-marquee-gap": `${ERROR_MARQUEE_GAP_PX}px`,
      } as React.CSSProperties;
    },
    [tickerOverflowDistance]
  );

  return (
    <>
      <div className="status-bar">
        <div className="status-left">
          <span className={`status-connection ${display.cssClass}${display.showPulse ? " pulsing" : ""}`}>
            <span className="led-dot" aria-hidden="true" />
            <span className="led-label">{display.label}</span>
          </span>
          {activeErrorMessage ? (
            <span className="status-error-wrap">
              <span className="chrome-sep" aria-hidden="true">·</span>
              <span
                ref={tickerViewportRef}
                className="status-error-viewport"
                title={activeErrorMessage}
              >
                <span
                  className={`status-error-marquee${
                    isTickerOverflowing ? " status-error-marquee--scrolling" : ""
                  }`}
                  style={tickerTrackStyle}
                >
                  <span ref={tickerSegmentRef} className="status-error-segment">
                    {activeErrorMessage}
                  </span>
                  {isTickerOverflowing ? (
                    <span className="status-error-segment" aria-hidden="true">
                      {activeErrorMessage}
                    </span>
                  ) : null}
                </span>
              </span>
            </span>
          ) : null}
        </div>

        <div className="status-right">
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

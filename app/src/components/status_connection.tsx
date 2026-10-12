// Path: app/src/components/status_connection.tsx
// Description: Status-strip connection LED and label plus the rotating, marquee-on-overflow error ticker

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type React from "react";
import { useAgent } from "../hooks/use_agent.js";
import { useConfig } from "../hooks/use_config.js";
import type { ConnectionStatus } from "../lib/agent/connection_state.js";

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

function useStatusErrors(): string[] {
  const { connectionState, helloState, agentError, agentDiagnostics } = useAgent();
  const { loadError, saveError, persistenceLocked, persistenceLockReason } = useConfig();
  const { status, lastError: connectionError } = connectionState;
  const agentErrorMessage = agentError
    ? `${agentError.message}${agentError.details?.docPath ? ` (See ${agentError.details.docPath})` : ""}`
    : null;
  // Connection errors while disconnected; agent/hello errors once connected.
  const errorToShow = agentDiagnostics
    ? null
    : status === "connected"
      ? agentErrorMessage ?? helloState.lastError
      : connectionError;
  const configErrorMessage = persistenceLocked
    ? `Config persistence locked: ${persistenceLockReason ?? "Config load failed"}`
    : loadError
      ? `Config load failed: ${loadError}`
      : saveError
        ? `Config save failed: ${saveError}`
        : null;
  return [errorToShow, configErrorMessage].filter((item): item is string => Boolean(item));
}

function ErrorTicker({ errorItems }: { errorItems: string[] }): React.JSX.Element | null {
  const [activeErrorIndex, setActiveErrorIndex] = useState(0);
  const [tickerOverflowDistance, setTickerOverflowDistance] = useState(0);
  const tickerViewportRef = useRef<HTMLSpanElement>(null);
  const tickerSegmentRef = useRef<HTMLSpanElement>(null);
  const errorSignature = errorItems.join("\u0001");

  useEffect(() => { setActiveErrorIndex(0); }, [errorSignature]);

  useEffect(() => {
    if (errorItems.length <= 1) return;
    const timer = window.setInterval(() => {
      setActiveErrorIndex((previous) => (errorItems.length === 0 ? 0 : (previous + 1) % errorItems.length));
    }, ERROR_ROTATION_INTERVAL_MS);
    return () => { window.clearInterval(timer); };
  }, [errorItems.length]);

  const activeErrorMessage = errorItems.length === 0
    ? null
    : errorItems[activeErrorIndex % errorItems.length] ?? errorItems[0] ?? null;

  const measureTickerOverflow = useCallback(() => {
    const viewport = tickerViewportRef.current;
    const segment = tickerSegmentRef.current;
    if (!viewport || !segment || !activeErrorMessage || segment.scrollWidth <= viewport.clientWidth) {
      setTickerOverflowDistance(0);
      return;
    }
    setTickerOverflowDistance(segment.scrollWidth + ERROR_MARQUEE_GAP_PX);
  }, [activeErrorMessage]);

  useLayoutEffect(() => { measureTickerOverflow(); }, [measureTickerOverflow]);

  useEffect(() => {
    if (!activeErrorMessage) return;
    window.addEventListener("resize", measureTickerOverflow);
    return () => { window.removeEventListener("resize", measureTickerOverflow); };
  }, [activeErrorMessage, measureTickerOverflow]);

  const tickerTrackStyle = useMemo(() => ({
    "--status-error-scroll-distance": `${tickerOverflowDistance}px`,
    "--status-error-scroll-duration": `${Math.max(6000, Math.round(tickerOverflowDistance * 28))}ms`,
    "--status-error-marquee-gap": `${ERROR_MARQUEE_GAP_PX}px`,
  }) as React.CSSProperties, [tickerOverflowDistance]);

  if (!activeErrorMessage) return null;
  const isTickerOverflowing = tickerOverflowDistance > 0;
  return (
    <span className="status-error-wrap">
      <span className="chrome-sep" aria-hidden="true">·</span>
      <span ref={tickerViewportRef} className="status-error-viewport" title={activeErrorMessage}>
        <span
          className={`status-error-marquee${isTickerOverflowing ? " status-error-marquee--scrolling" : ""}`}
          style={tickerTrackStyle}
        >
          <span ref={tickerSegmentRef} className="status-error-segment">{activeErrorMessage}</span>
          {isTickerOverflowing ? (
            <span className="status-error-segment" aria-hidden="true">{activeErrorMessage}</span>
          ) : null}
        </span>
      </span>
    </span>
  );
}

export function StatusConnection(): React.JSX.Element {
  const { connectionState } = useAgent();
  const display = getConnectionDisplay(connectionState.status, connectionState.reconnectAttempts);
  const errorItems = useStatusErrors();
  return (
    <div className="status-left">
      <span
        className={`status-connection ${display.cssClass}${display.showPulse ? " pulsing" : ""}`}
        title={display.label}
      >
        <span className="led-dot" aria-hidden="true" />
        <span className="led-label">{display.label}</span>
      </span>
      <ErrorTicker errorItems={errorItems} />
    </div>
  );
}

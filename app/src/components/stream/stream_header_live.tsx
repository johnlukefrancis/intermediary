// Path: app/src/components/stream/stream_header_live.tsx
// Description: The Stream header's LIVE indicator: one dot plus a mono state label

import type React from "react";

export type StreamLiveState = "live" | "held" | "offline" | "update";

const LABELS: Record<StreamLiveState, string> = {
  live: "LIVE",
  held: "HELD",
  offline: "OFFLINE",
  update: "UPDATE",
};

const TITLES: Record<StreamLiveState, string> = {
  live: "Stream is live",
  held: "Stream is held",
  offline: "Agent is not connected",
  update: "Agent update required for the stream",
};

/**
 * Colour is never the only channel: the label carries the same state as the dot, and the
 * title exposes it to hover and to assistive tech (the span itself is decorative).
 */
export function StreamHeaderLive({ state }: { state: StreamLiveState }): React.JSX.Element {
  return (
    <span className="stream-live" data-state={state} title={TITLES[state]}>
      <span className="stream-live__dot" aria-hidden="true" />
      <span className="stream-live__label">{LABELS[state]}</span>
    </span>
  );
}

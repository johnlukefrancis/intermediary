// Path: app/src/components/stream/stream_header_status.tsx
// Description: Stream header status when observation is interrupted or unavailable

import type React from "react";

export type StreamStatus = "held" | "offline" | "update";

const LABELS: Record<StreamStatus, string> = {
  held: "HELD",
  offline: "OFFLINE",
  update: "UPDATE",
};

const TITLES: Record<StreamStatus, string> = {
  held: "Stream is held",
  offline: "Agent is not connected",
  update: "Agent update required for the stream",
};

export function StreamHeaderStatus({ state }: { state: StreamStatus }): React.JSX.Element {
  return (
    <span className="stream-status" data-state={state} title={TITLES[state]}>
      {LABELS[state]}
    </span>
  );
}

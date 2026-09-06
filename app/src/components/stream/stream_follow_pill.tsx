// Path: app/src/components/stream/stream_follow_pill.tsx
// Description: Sticky "N NEW" button shown while the reader has scrolled away from the live tail

import type React from "react";
import { CaretDownIcon } from "./stream_icons.js";

interface StreamFollowPillProps {
  unread: number;
  onResume: () => void;
}

export function StreamFollowPill({ unread, onResume }: StreamFollowPillProps): React.JSX.Element {
  const label = unread === 1 ? "1 NEW" : `${String(unread)} NEW`;
  return (
    <button
      type="button"
      className="stream-follow-pill"
      onClick={onResume}
      aria-label={`${label} — jump to the latest edit`}
      title="Jump to the latest edit"
    >
      <CaretDownIcon />
      <span>{label}</span>
    </button>
  );
}

// Path: app/src/components/stream/stream_notice_row.tsx
// Description: Console-prompt notice row: honesty counters and transport state printed inline in the stream

import type React from "react";
import type { StreamNoticeRow as StreamNotice } from "../../lib/stream/stream_types.js";

export function StreamNoticeRow({ notice }: { notice: StreamNotice }): React.JSX.Element {
  return (
    <div className="stream-notice" data-tone={notice.tone} role="note">
      {notice.text}
    </div>
  );
}

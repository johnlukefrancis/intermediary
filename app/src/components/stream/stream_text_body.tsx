// Path: app/src/components/stream/stream_text_body.tsx
// Description: Capped diff lines in the shared row grammar with the "+N MORE · OPEN DIFF" and "NEW FILE" footers

import type React from "react";
import { DiffLineRows } from "../diff/diff_line_rows.js";
import { lineCap, selectLines } from "../../lib/stream/stream_card_grammar.js";
import type { StreamFileCard, StreamTextBody as TextBody } from "../../lib/stream/stream_types.js";

interface StreamTextBodyProps {
  card: StreamFileCard;
  body: TextBody;
  handset: boolean;
  onOpen: (card: StreamFileCard) => void;
}

export function StreamTextBody({ card, body, handset, onOpen }: StreamTextBodyProps): React.JSX.Element {
  const selection = selectLines(body.segments, card.expanded, lineCap(handset));
  const { lines } = selection;
  // What the cap cut is still hidden content the footer must own up to
  const hiddenLines = selection.hiddenLines + body.beyondCap;
  const showOpenDiff = card.tracked !== false && card.op !== "add";
  const newFile = card.op === "add" && body.baseline === "none";
  // Only the newest segment's rows are fresh on a merged card; a single edit prints whole
  const freshFrom = card.edits > 1 ? selection.newestFrom : undefined;

  return (
    <>
      <div className="stream-card__lines">
        <DiffLineRows lines={lines} staggerIndex freshFrom={freshFrom} />
      </div>
      {(hiddenLines > 0 || showOpenDiff || newFile) && (
        <footer className="stream-card__foot">
          {newFile && <span className="stream-card__foot-note">{`NEW FILE · ${String(body.stats.newLines)} LINES`}</span>}
          {hiddenLines > 0 && (
            <span className="stream-card__foot-note">{card.expanded ? `+${String(hiddenLines)} BEYOND CAP` : `+${String(hiddenLines)} MORE`}</span>
          )}
          {showOpenDiff && (
            <button
              type="button"
              className="stream-card__foot-action"
              onClick={(event) => {
                event.stopPropagation();
                onOpen(card);
              }}
            >
              OPEN DIFF
            </button>
          )}
        </footer>
      )}
    </>
  );
}

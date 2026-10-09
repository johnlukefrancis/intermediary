// Path: app/src/components/overview/overview_settled_edit.tsx
// Description: Observed edit with Stream-family file identity and a six-line excerpt

import type React from "react";
import type { OverviewEdit } from "../../lib/overview/overview_types.js";
import type { DeltaBaseline } from "../../shared/protocol.js";
import type { ImageTile } from "../../hooks/images/use_image_tiles.js";
import { previewableImage } from "../../lib/images/image_tile_pixels.js";
import { DiffLineRows } from "../diff/diff_line_rows.js";
import { OverviewAge, OverviewFileHead } from "./overview_file_head.js";
import { OverviewImagePreview } from "./overview_image_preview.js";

const BASELINE_CHIPS: Readonly<Record<DeltaBaseline, { label: string; title: string }>> = {
  previousSighting: { label: "SINCE LAST", title: "Compared with the agent's previous sighting" },
  index: { label: "VS INDEX", title: "Compared with the staged index" },
  none: { label: "NEW", title: "No earlier baseline was available" },
};

export function hasImagePreview(edit: OverviewEdit): boolean {
  return edit.image !== null && edit.op !== "remove"
    && previewableImage(edit.path, edit.image.mimeType, edit.image.bytes);
}

export function OverviewSettledEdit({ edit, now, imageTile, onOpen }: { edit: OverviewEdit; now: number; imageTile: ImageTile | undefined; onOpen: () => void }): React.JSX.Element {
  const chip = edit.baseline === null ? null : BASELINE_CHIPS[edit.baseline];
  return (
    <section className="overview-edit" aria-label="File edit" data-edit-id={edit.id}>
      <OverviewFileHead path={edit.path} op={edit.op} fromPath={edit.fromPath} onOpen={onOpen} age={<OverviewAge at={edit.at} now={now} />} meta={
        <>
          {edit.added !== null && edit.removed !== null && edit.added + edit.removed > 0 && (
            <span className="overview-edit__stats"><span data-stat="add">+{edit.added}</span><span data-stat="del">−{edit.removed}</span></span>
          )}
          {edit.truncated && <span className="badge badge--warning" title="Bounded excerpt; open Stream for available detail">TRUNC</span>}
          {chip && <span className="overview-edit__chip" title={chip.title}>{chip.label}</span>}
        </>
      } />
      {hasImagePreview(edit) ? <OverviewImagePreview path={edit.path} tile={imageTile} onOpen={onOpen} />
        : edit.lines.length > 0 ? <div className="overview-edit__lines" aria-label="Edit preview, up to six lines">
          <DiffLineRows lines={edit.lines.slice(0, 6).map((line) => ({ ...line, oldNo: null, newNo: null }))} />
        </div> : <p className="overview-edit__detail">{edit.detail}</p>}
    </section>
  );
}

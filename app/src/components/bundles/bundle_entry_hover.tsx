// Path: app/src/components/bundles/bundle_entry_hover.tsx
// Description: Glass inspection tooltip with source sizes and largest selected files

import type React from "react";
import { useLayoutEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { formatBytes } from "../../lib/format_bytes.js";
import type { BundleEntryInspection } from "../../shared/protocol_bundle_inspection.js";
import "../../styles/context_menu.css";
import "../../styles/bundle_entry_hover.css";

export interface BundleHoverState {
  path: string;
  anchor: DOMRect;
  inspection: BundleEntryInspection | null;
  error: string | null;
}

export function BundleEntryHover({ id, hover }: { id: string; hover: BundleHoverState }): React.JSX.Element {
  const ref = useRef<HTMLDivElement>(null);
  const { path, anchor, inspection, error } = hover;
  useLayoutEffect(() => {
    const panel = ref.current;
    if (!panel) return;
    const width = panel.offsetWidth;
    const height = panel.offsetHeight;
    const left = anchor.left >= width + 16 ? anchor.left - width - 8 : anchor.right + 8;
    panel.style.left = `${Math.max(8, Math.min(left, window.innerWidth - width - 8))}px`;
    panel.style.top = `${Math.max(8, Math.min(anchor.top, window.innerHeight - height - 8))}px`;
  }, [anchor, inspection, error]);
  const size = (bytes: number): string => `${inspection?.complete === false ? "≥ " : ""}${formatBytes(bytes)}`;
  return createPortal(
    <div ref={ref} id={id} role="tooltip" className="glass-popover bundle-entry-hover">
      <div className="bundle-entry-hover__path">{path}</div>
      {error ? <p className="bundle-entry-hover__note">{error}</p> : !inspection ? (
        <p className="bundle-entry-hover__note">Measuring size…</p>
      ) : <>
        <dl className="bundle-entry-hover__sizes">
          <div><dt>{inspection.isDirectory ? "Folder size" : "File size"}</dt><dd>{size(inspection.bytes)}</dd></div>
          <div className="bundle-entry-hover__included"><dt>Selected for bundle</dt><dd>{size(inspection.includedBytes)}</dd></div>
        </dl>
        {inspection.isDirectory && <p className="bundle-entry-hover__note">
          {inspection.includedFileCount.toLocaleString()} of {inspection.fileCount.toLocaleString()} files selected
        </p>}
        {!inspection.isDirectory && <p className="bundle-entry-hover__note">
          {inspection.includedFileCount ? "Included" : "Excluded from bundle"}
          {inspection.modifiedMs !== null && ` · Modified ${new Date(inspection.modifiedMs).toLocaleString()}`}
        </p>}
        {inspection.isDirectory && inspection.largestIncludedFiles.length > 0 && <>
          <div className="bundle-entry-hover__heading">Largest selected files</div>
          <ol className="bundle-entry-hover__largest">
            {inspection.largestIncludedFiles.map((file) => <li key={file.path}>
              <span>{file.path.slice(path.length + 1)}</span><strong>{formatBytes(file.bytes)}</strong>
            </li>)}
          </ol>
        </>}
        {!inspection.complete && <p className="bundle-entry-hover__note bundle-entry-hover__partial">
          Partial scan · sizes are lower bounds; largest files shown are from scanned entries.
          {inspection.unreadableEntries > 0 && ` ${inspection.unreadableEntries} unreadable entries.`}
        </p>}
      </>}
    </div>, document.body
  );
}

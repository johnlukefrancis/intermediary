// Path: app/src/components/overview/overview_file_head.tsx
// Description: Shared Overview filename-over-directory typography, icon and operation badge

import type React from "react";
import { baseName, parentPath } from "../../lib/bundles/bundle_selection_visibility.js";
import { FileIcon, getFileFamily } from "../../lib/icons/index.js";
import { CHANGE_BADGES } from "../../lib/source_control/change_badges.js";
import type { DeltaOp, SourceControlChange } from "../../shared/protocol.js";
import { relativeAge } from "./overview_presentation.js";

const OP_BADGES: Readonly<Record<DeltaOp, SourceControlChange>> = {
  add: "added", modify: "modified", remove: "deleted", rename: "renamed",
};

interface OverviewFileHeadProps {
  path: string;
  op: DeltaOp | null;
  onOpen: () => void;
  fromPath?: string | null;
  meta?: React.ReactNode;
  age: React.ReactNode;
}

export function OverviewFileHead({ path, op, onOpen, fromPath, meta, age }: OverviewFileHeadProps): React.JSX.Element {
  const badge = op === null ? null : CHANGE_BADGES[OP_BADGES[op]];
  const dir = parentPath(path);
  return (
    <div className="overview-file" data-history={op === null}>
      {badge && <span className={`badge badge--${badge.variant}`} title={badge.label}>{badge.letter}</span>}
      <span className="overview-file__icon"><FileIcon family={getFileFamily(path)} /></span>
      <button type="button" className="overview-file__path" draggable onClick={onOpen}
        aria-label={`Open ${baseName(path)}`} title={fromPath ? `${fromPath} → ${path}` : path}>
        <span className="overview-file__name">{fromPath && <span className="overview-file__from">{baseName(fromPath)} → </span>}{baseName(path)}</span>
        {dir && <span className="overview-file__dir">{dir}</span>}
      </button>
      <span className="overview-file__meta">{meta}</span>
      {age}
    </div>
  );
}

export function OverviewAge({ at, now }: { at: number; now: number }): React.JSX.Element {
  return <span className="overview-entry-age" title={new Date(at).toLocaleString()}>{relativeAge(at, now)}</span>;
}

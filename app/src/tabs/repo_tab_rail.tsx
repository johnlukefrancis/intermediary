// Path: app/src/tabs/repo_tab_rail.tsx
// Description: Composes the ZIPS, SOURCE and TERMINAL rail bodies for one repo tab; RepoTab hands them to the rail or the handset deck

import { BundleColumn } from "../components/bundles/bundle_column.js";
import type { RailBodies } from "../components/layout/repo_rail.js";
import { SourceControlColumn } from "../components/source_control/source_control_column.js";
import { TerminalColumn } from "../components/terminal/terminal_column.js";
import type { SourceControlState } from "../hooks/source_control/source_control_types.js";
import { TreeDecorationsProvider } from "../hooks/source_control/use_tree_decorations.js";
import type { BundleState } from "../hooks/use_bundle_state.js";
import type { RepoRoot } from "../shared/config.js";
import type { SourceControlEntry } from "../shared/protocol.js";

export interface RepoRailBodiesInput {
  repoId: string;
  /** Undefined while the repo is not in the config; the terminal cannot open then */
  repoRoot: RepoRoot | undefined;
  isConnected: boolean;
  bundleState: BundleState;
  topLevelFiles: string[];
  sourceControl: SourceControlState;
  onBundleDragStart: (hostPath: string) => Promise<void>;
  onOpenFile: (path: string) => void;
  onOpenDiff: (entry: SourceControlEntry) => void;
}

/** All three bodies are built every render; only the active one is mounted by the rail */
export function buildRepoRailBodies({
  repoId,
  repoRoot,
  isConnected,
  bundleState,
  topLevelFiles,
  sourceControl,
  onBundleDragStart,
  onOpenFile,
  onOpenDiff,
}: RepoRailBodiesInput): RailBodies {
  return {
    zips: (
      <TreeDecorationsProvider status={sourceControl.status}>
        <BundleColumn
          repoId={repoId}
          bundleState={bundleState}
          topLevelFiles={topLevelFiles}
          onDragStart={onBundleDragStart}
          onOpenFile={onOpenFile}
          emptyMessage={!isConnected ? "Waiting for agent..." : "No bundles yet"}
        />
      </TreeDecorationsProvider>
    ),
    source: (
      <SourceControlColumn repoId={repoId} state={sourceControl} onOpenDiff={onOpenDiff} />
    ),
    terminal: <TerminalColumn repoId={repoId} repoRoot={repoRoot} />,
  };
}

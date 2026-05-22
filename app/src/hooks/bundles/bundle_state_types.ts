// Path: app/src/hooks/bundles/bundle_state_types.ts
// Description: Bundle state contracts shared by bundle hooks and UI

import type {
  BundleBuildPhase,
  BundleInfo,
  BundleSelection,
} from "../../shared/protocol.js";

export interface BundleBuildProgress {
  phase: BundleBuildPhase;
  filesDone: number;
  filesTotal: number;
  currentFile?: string;
  currentBytesDone?: number;
  currentBytesTotal?: number;
  bytesDoneTotalBestEffort?: number;
}

export interface BundlePresetState {
  presetId: string;
  presetName: string;
  selection: BundleSelection;
  isSelectionInitialized: boolean;
  isBuilding: boolean;
  isCancelling: boolean;
  activeBuildId: string | null;
  buildProgress: BundleBuildProgress | null;
  bundles: BundleInfo[];
  lastBuildError: string | null;
  /** Timestamp (ms) when bundle was last built, for fresh pulse animation */
  freshlyBuiltAt: number | null;
}

export interface BundleState {
  presets: Map<string, BundlePresetState>;
  activePresetId: string;
  topLevelDirs: string[];
  topLevelSubdirs: Record<string, string[]>;
  setSelection: (presetId: string, selection: BundleSelection) => void;
  buildBundle: (presetId: string) => Promise<void>;
  cancelBundleBuild: (presetId: string) => Promise<void>;
  setActivePreset: (presetId: string) => void;
  refreshBundles: (presetId: string) => Promise<void>;
}

export interface BundleProgressThrottleEntry {
  ts: number;
  phase: BundleBuildPhase;
  filesDone: number;
  filesTotal: number;
  currentFile?: string;
}

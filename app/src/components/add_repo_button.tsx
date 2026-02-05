// Path: app/src/components/add_repo_button.tsx
// Description: "+" button for adding new repositories via directory picker

import type React from "react";
import { useCallback, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useConfig } from "../hooks/use_config.js";
import {
  extractFolderName,
  generateUniqueRepoId,
} from "../shared/repo_utils.js";
import {
  type RepoConfig,
  type RepoRoot,
  DEFAULT_DOCS_GLOBS,
  DEFAULT_CODE_GLOBS,
  DEFAULT_IGNORE_GLOBS,
  DEFAULT_BUNDLE_PRESET,
  repoRootKey,
} from "../shared/config.js";

interface AddRepoButtonProps {
  onRepoAdded?: (repoId: string) => void;
  className?: string;
}

export function AddRepoButton({
  onRepoAdded,
  className = "",
}: AddRepoButtonProps): React.JSX.Element {
  const { config, addRepo } = useConfig();
  const [isAdding, setIsAdding] = useState(false);

  const handleClick = useCallback(async () => {
    if (isAdding) return;
    setIsAdding(true);

    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) {
        return;
      }

      const root = await invoke<RepoRoot>("resolve_repo_root", {
        inputPath: selected,
      });

      // Check for duplicates
      const existingPaths = new Set(config.repos.map((r) => repoRootKey(r.root)));
      if (existingPaths.has(repoRootKey(root))) {
        console.warn("[AddRepoButton] Repository already exists:", root);
        return;
      }

      // Generate unique repoId
      const folderName = extractFolderName(root.path);
      const existingIds = new Set(config.repos.map((r) => r.repoId));
      const repoId = generateUniqueRepoId(folderName, existingIds);

      const newRepo: RepoConfig = {
        repoId,
        label: folderName,
        root,
        autoStage: true,
        docsGlobs: DEFAULT_DOCS_GLOBS,
        codeGlobs: DEFAULT_CODE_GLOBS,
        ignoreGlobs: DEFAULT_IGNORE_GLOBS,
        bundlePresets: [DEFAULT_BUNDLE_PRESET],
      };

      addRepo(newRepo);
      onRepoAdded?.(repoId);
    } catch (err) {
      console.error("[AddRepoButton] Failed to add repo:", err);
    } finally {
      setIsAdding(false);
    }
  }, [isAdding, config.repos, addRepo, onRepoAdded]);

  const onClick = useCallback(() => {
    void handleClick();
  }, [handleClick]);

  return (
    <button
      type="button"
      className={`tab-add-button ${className}`}
      onClick={onClick}
      disabled={isAdding}
      aria-label="Add repository"
      title="Add repository"
    >
      +
    </button>
  );
}

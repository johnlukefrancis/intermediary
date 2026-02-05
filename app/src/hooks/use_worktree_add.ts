// Path: app/src/hooks/use_worktree_add.ts
// Description: Hook for adding worktrees to existing groups or single repos

import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { SingleTab } from "../app.js";
import { useConfig } from "./use_config.js";
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

const ADD_ERROR_TIMEOUT_MS = 3000;

type OpenSelection = string | string[] | null;

interface UseWorktreeAddOptions {
  onRepoChange: (repoId: string) => void;
  onRepoAdded?: (repoId: string) => void;
}

interface WorktreeAddResult {
  isAdding: boolean;
  addError: string | null;
  addWorktreeToGroup: (groupId: string, groupLabel: string) => Promise<boolean>;
  addWorktreeToSingle: (tab: SingleTab) => Promise<boolean>;
}

function resolveSingleSelection(selection: OpenSelection): string | null {
  if (!selection) return null;
  if (Array.isArray(selection)) return selection[0] ?? null;
  return selection;
}

function buildRepoConfig(
  repoId: string,
  label: string,
  root: RepoRoot,
  groupId?: string,
  groupLabel?: string
): RepoConfig {
  return {
    repoId,
    label,
    root,
    groupId,
    groupLabel,
    autoStage: true,
    docsGlobs: DEFAULT_DOCS_GLOBS,
    codeGlobs: DEFAULT_CODE_GLOBS,
    ignoreGlobs: DEFAULT_IGNORE_GLOBS,
    bundlePresets: [DEFAULT_BUNDLE_PRESET],
  };
}

export function useWorktreeAdd({
  onRepoChange,
  onRepoAdded,
}: UseWorktreeAddOptions): WorktreeAddResult {
  const { config, addRepo, updateRepo } = useConfig();
  const [isAdding, setIsAdding] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);

  useEffect(() => {
    if (!addError) return;
    const timer = setTimeout(() => {
      setAddError(null);
    }, ADD_ERROR_TIMEOUT_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [addError]);

  const resolveRepoRoot = useCallback(async (): Promise<RepoRoot | null> => {
    const selection = await open({ directory: true, multiple: false });
    const inputPath = resolveSingleSelection(selection);
    if (!inputPath) return null;

    return invoke<RepoRoot>("resolve_repo_root", {
      inputPath,
    });
  }, []);

  const ensureUniquePath = useCallback(
    (root: RepoRoot): boolean => {
      const existingPaths = new Set(config.repos.map((r) => repoRootKey(r.root)));
      if (existingPaths.has(repoRootKey(root))) {
        setAddError("This folder is already added");
        return false;
      }
      return true;
    },
    [config.repos]
  );

  const createUniqueRepoId = useCallback(
    (folderName: string): string => {
      const existingIds = new Set(config.repos.map((r) => r.repoId));
      return generateUniqueRepoId(folderName, existingIds);
    },
    [config.repos]
  );

  const addWorktreeToGroup = useCallback(
    async (groupId: string, groupLabel: string): Promise<boolean> => {
      if (isAdding) return false;
      setIsAdding(true);
      setAddError(null);

      try {
        const root = await resolveRepoRoot();
        if (!root) return false;
        if (!ensureUniquePath(root)) return false;

        const folderName = extractFolderName(root.path);
        const repoId = createUniqueRepoId(folderName);
        const newRepo = buildRepoConfig(
          repoId,
          folderName,
          root,
          groupId,
          groupLabel
        );

        addRepo(newRepo);
        onRepoChange(repoId);
        onRepoAdded?.(repoId);
        return true;
      } catch (err) {
        console.error("[useWorktreeAdd] Failed to add worktree:", err);
        setAddError("Failed to add worktree");
        return false;
      } finally {
        setIsAdding(false);
      }
    },
    [
      isAdding,
      resolveRepoRoot,
      ensureUniquePath,
      createUniqueRepoId,
      addRepo,
      onRepoChange,
      onRepoAdded,
    ]
  );

  const addWorktreeToSingle = useCallback(
    async (tab: SingleTab): Promise<boolean> => {
      if (isAdding) return false;
      setIsAdding(true);
      setAddError(null);

      try {
        const root = await resolveRepoRoot();
        if (!root) return false;
        if (!ensureUniquePath(root)) return false;

        const groupId = tab.repoId;
        const groupLabel = tab.label;
        updateRepo(tab.repoId, { groupId, groupLabel });

        const folderName = extractFolderName(root.path);
        const repoId = createUniqueRepoId(folderName);
        const newRepo = buildRepoConfig(
          repoId,
          folderName,
          root,
          groupId,
          groupLabel
        );

        addRepo(newRepo);
        onRepoChange(repoId);
        onRepoAdded?.(repoId);
        return true;
      } catch (err) {
        console.error("[useWorktreeAdd] Failed to add worktree:", err);
        setAddError("Failed to add worktree");
        return false;
      } finally {
        setIsAdding(false);
      }
    },
    [
      isAdding,
      resolveRepoRoot,
      ensureUniquePath,
      createUniqueRepoId,
      addRepo,
      updateRepo,
      onRepoChange,
      onRepoAdded,
    ]
  );

  return {
    isAdding,
    addError,
    addWorktreeToGroup,
    addWorktreeToSingle,
  };
}

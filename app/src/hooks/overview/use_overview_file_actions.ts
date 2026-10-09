// Path: app/src/hooks/overview/use_overview_file_actions.ts
// Description: Repository-bound Overview context menus and user-triggered drag-out actions

import type React from "react";
import { useCallback, useEffect, useState } from "react";
import type { RepoConfig } from "../../shared/config.js";
import { buildSingleFileContextMenuItems } from "../../components/file_context_menu_items.js";
import { useFileActions } from "../use_file_actions.js";
import { useDrag } from "../use_drag.js";

interface MenuTarget { repo: RepoConfig; path: string; x: number; y: number }

export function useOverviewFileActions(repos: readonly RepoConfig[], documentHidden: boolean) {
  const fileActions = useFileActions();
  const { dragState, handleDragStart, clearError } = useDrag();
  const [target, setTarget] = useState<MenuTarget | null>(null);
  const current = target === null ? undefined : repos.find((repo) => repo.repoId === target.repo.repoId);
  const valid = target !== null && !documentHidden && current !== undefined
    && current.root.kind === target.repo.root.kind && current.root.path === target.repo.root.path;
  useEffect(() => {
    if (target !== null && !valid) setTarget(null);
  }, [target, valid]);
  const closeMenu = useCallback(() => { setTarget(null); }, []);
  const openMenu = useCallback((event: React.MouseEvent, repo: RepoConfig, path: string) => {
    event.preventDefault();
    event.stopPropagation();
    const bounds = event.currentTarget.getBoundingClientRect();
    setTarget({ repo, path, x: event.clientX || bounds.left + 12, y: event.clientY || bounds.top + 12 });
  }, []);
  const dragFile = useCallback((repo: RepoConfig, path: string) => {
    void handleDragStart(repo.repoId, path, undefined);
  }, [handleDragStart]);
  const menu = target === null || !valid ? null : {
    x: target.x, y: target.y,
    items: buildSingleFileContextMenuItems({ repoRoot: target.repo.root, path: target.path, fileActions, logScope: "Overview" }),
  };
  return { menu, openMenu, closeMenu, dragFile, dragState, clearError };
}

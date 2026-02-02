// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation with grouped repo dropdown support

import type React from "react";
import { useState, useCallback, useEffect, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { TabItem, SingleTab, GroupTab } from "../app.js";
import { useConfig } from "../hooks/use_config.js";
import { AddRepoButton } from "./add_repo_button.js";
import { TabRemoveButton } from "./tab_remove_button.js";
import {
  extractFolderName,
  generateUniqueRepoId,
} from "../shared/repo_utils.js";
import {
  type RepoConfig,
  DEFAULT_DOCS_GLOBS,
  DEFAULT_CODE_GLOBS,
  DEFAULT_IGNORE_GLOBS,
  DEFAULT_BUNDLE_PRESET,
} from "../shared/config.js";
import "../styles/tab_bar.css";

interface TabBarProps {
  tabs: TabItem[];
  activeRepoId: string | null;
  onRepoChange: (repoId: string) => void;
  onRepoAdded?: (repoId: string) => void;
}

/** Check if the active repo belongs to this group */
function isGroupActive(group: GroupTab, activeRepoId: string | null): boolean {
  return group.repos.some((r) => r.repoId === activeRepoId);
}

/** Get the active repo's label within a group */
function getActiveRepoLabel(group: GroupTab, activeRepoId: string | null): string {
  const repo = group.repos.find((r) => r.repoId === activeRepoId);
  return repo?.label ?? group.repos[0]?.label ?? "";
}

export function TabBar({ tabs, activeRepoId, onRepoChange, onRepoAdded }: TabBarProps): React.JSX.Element {
  const { config, addRepo, updateRepo } = useConfig();
  const [openDropdownId, setOpenDropdownId] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Clear error after timeout
  useEffect(() => {
    if (!addError) return;
    const timer = setTimeout(() => { setAddError(null); }, 3000);
    return () => { clearTimeout(timer); };
  }, [addError]);

  // Close dropdown on outside click
  useEffect(() => {
    if (!openDropdownId) return;

    function handleClickOutside(e: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setOpenDropdownId(null);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [openDropdownId]);

  const handleGroupClick = useCallback(
    (group: GroupTab) => {
      // If clicking the group tab, select first repo in group (or current if already in group)
      const isActive = isGroupActive(group, activeRepoId);
      if (!isActive) {
        const firstRepo = group.repos[0];
        if (firstRepo) {
          onRepoChange(firstRepo.repoId);
        }
      }
    },
    [activeRepoId, onRepoChange]
  );

  const toggleDropdown = useCallback(
    (e: React.MouseEvent, dropdownId: string) => {
      e.stopPropagation();
      setOpenDropdownId((prev) => (prev === dropdownId ? null : dropdownId));
    },
    []
  );

  const handleRepoSelect = useCallback(
    (repoId: string) => {
      onRepoChange(repoId);
      setOpenDropdownId(null);
    },
    [onRepoChange]
  );

  /** Add worktree to an existing group */
  const handleAddWorktreeToGroup = useCallback(
    async (groupId: string, groupLabel: string) => {
      if (isAdding) return;
      setIsAdding(true);
      setAddError(null);

      try {
        const selected = await open({ directory: true, multiple: false });
        if (!selected) return;

        const wslPath = await invoke<string>("convert_windows_to_wsl", {
          windowsPath: selected,
        });

        // Check for duplicate
        const existingPaths = new Set(config.repos.map((r) => r.wslPath));
        if (existingPaths.has(wslPath)) {
          setAddError("This folder is already added");
          return;
        }

        // Generate unique repoId
        const folderName = extractFolderName(wslPath);
        const existingIds = new Set(config.repos.map((r) => r.repoId));
        const repoId = generateUniqueRepoId(folderName, existingIds);

        const newRepo: RepoConfig = {
          repoId,
          label: folderName,
          wslPath,
          groupId,
          groupLabel,
          autoStage: true,
          docsGlobs: DEFAULT_DOCS_GLOBS,
          codeGlobs: DEFAULT_CODE_GLOBS,
          ignoreGlobs: DEFAULT_IGNORE_GLOBS,
          bundlePresets: [DEFAULT_BUNDLE_PRESET],
        };

        addRepo(newRepo);
        onRepoChange(repoId);
        onRepoAdded?.(repoId);
        setOpenDropdownId(null);
      } catch (err) {
        console.error("[TabBar] Failed to add worktree:", err);
        setAddError("Failed to add worktree");
      } finally {
        setIsAdding(false);
      }
    },
    [isAdding, config.repos, addRepo, onRepoChange, onRepoAdded]
  );

  /** Convert single tab to group and add worktree */
  const handleAddWorktreeToSingle = useCallback(
    async (singleTab: SingleTab) => {
      if (isAdding) return;
      setIsAdding(true);
      setAddError(null);

      try {
        const selected = await open({ directory: true, multiple: false });
        if (!selected) return;

        const wslPath = await invoke<string>("convert_windows_to_wsl", {
          windowsPath: selected,
        });

        // Check for duplicate
        const existingPaths = new Set(config.repos.map((r) => r.wslPath));
        if (existingPaths.has(wslPath)) {
          setAddError("This folder is already added");
          return;
        }

        // Generate groupId from original repo's repoId
        const groupId = singleTab.repoId;
        const groupLabel = singleTab.label;

        // Update original repo to have groupId/groupLabel
        updateRepo(singleTab.repoId, { groupId, groupLabel });

        // Generate unique repoId for new worktree
        const folderName = extractFolderName(wslPath);
        const existingIds = new Set(config.repos.map((r) => r.repoId));
        const repoId = generateUniqueRepoId(folderName, existingIds);

        const newRepo: RepoConfig = {
          repoId,
          label: folderName,
          wslPath,
          groupId,
          groupLabel,
          autoStage: true,
          docsGlobs: DEFAULT_DOCS_GLOBS,
          codeGlobs: DEFAULT_CODE_GLOBS,
          ignoreGlobs: DEFAULT_IGNORE_GLOBS,
          bundlePresets: [DEFAULT_BUNDLE_PRESET],
        };

        addRepo(newRepo);
        onRepoChange(repoId);
        onRepoAdded?.(repoId);
        setOpenDropdownId(null);
      } catch (err) {
        console.error("[TabBar] Failed to add worktree:", err);
        setAddError("Failed to add worktree");
      } finally {
        setIsAdding(false);
      }
    },
    [isAdding, config.repos, addRepo, updateRepo, onRepoChange, onRepoAdded]
  );

  return (
    <nav className="tab-bar">
      {tabs.map((tab) => {
        if (tab.type === "single") {
          const isActive = activeRepoId === tab.repoId;
          const isOpen = openDropdownId === tab.repoId;

          return (
            <div
              key={tab.repoId}
              className="single-tab-container"
              ref={isOpen ? dropdownRef : undefined}
            >
              <button
                className={`tab-button ${isActive ? "active" : ""}`}
                onClick={() => { onRepoChange(tab.repoId); }}
                type="button"
                aria-current={isActive ? "page" : undefined}
              >
                <span className="tab-label">{tab.label}</span>
              </button>
              <button
                className="group-chevron"
                onClick={(e) => { toggleDropdown(e, tab.repoId); }}
                type="button"
                aria-expanded={isOpen}
                aria-label={`Add worktree to ${tab.label}`}
              >
                {isOpen ? "▲" : "▼"}
              </button>
              <TabRemoveButton
                repoId={tab.repoId}
                label={tab.label}
              />

              {isOpen && (
                <div className="group-dropdown">
                  {addError && (
                    <div className="group-dropdown-error">{addError}</div>
                  )}
                  <button
                    className="group-dropdown-add"
                    onClick={() => { void handleAddWorktreeToSingle(tab); }}
                    disabled={isAdding}
                    type="button"
                  >
                    + Add worktree
                  </button>
                </div>
              )}
            </div>
          );
        } else {
          const isActive = isGroupActive(tab, activeRepoId);
          const activeLabel = getActiveRepoLabel(tab, activeRepoId);
          const isOpen = openDropdownId === tab.groupId;

          return (
            <div
              key={tab.groupId}
              className="group-tab-container"
              ref={isOpen ? dropdownRef : undefined}
            >
              <button
                className={`tab-button ${isActive ? "active" : ""}`}
                onClick={() => { handleGroupClick(tab); }}
                type="button"
                aria-current={isActive ? "page" : undefined}
              >
                <span className="tab-label">
                  {tab.groupLabel}: {activeLabel}
                </span>
              </button>
              <button
                className="group-chevron"
                onClick={(e) => { toggleDropdown(e, tab.groupId); }}
                type="button"
                aria-expanded={isOpen}
                aria-label={`Select ${tab.groupLabel} worktree`}
              >
                {isOpen ? "▲" : "▼"}
              </button>

              {isOpen && (
                <div className="group-dropdown">
                  {tab.repos.map((repo) => {
                    const isSelected = activeRepoId === repo.repoId;
                    return (
                      <div key={repo.repoId} className="group-dropdown-row">
                        <button
                          className={`group-dropdown-item ${isSelected ? "selected" : ""}`}
                          onClick={() => { handleRepoSelect(repo.repoId); }}
                          type="button"
                        >
                          <span className="group-radio">{isSelected ? "●" : "○"}</span>
                          {repo.label}
                        </button>
                        <TabRemoveButton
                          repoId={repo.repoId}
                          label={repo.label}
                          className="group-dropdown-remove"
                        />
                      </div>
                    );
                  })}
                  <div className="group-dropdown-divider" />
                  {addError && (
                    <div className="group-dropdown-error">{addError}</div>
                  )}
                  <button
                    className="group-dropdown-add"
                    onClick={() => { void handleAddWorktreeToGroup(tab.groupId, tab.groupLabel); }}
                    disabled={isAdding}
                    type="button"
                  >
                    + Add worktree
                  </button>
                </div>
              )}
            </div>
          );
        }
      })}
      <AddRepoButton {...(onRepoAdded ? { onRepoAdded } : {})} />
    </nav>
  );
}

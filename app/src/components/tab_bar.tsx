// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation with grouped repo dropdown support

import type React from "react";
import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TabItem, SingleTab, GroupTab } from "../app.js";
import { useWorktreeAdd } from "../hooks/use_worktree_add.js";
import { AddRepoButton } from "./add_repo_button.js";
import { TabRemoveButton } from "./tab_remove_button.js";
import { GroupRemoveButton } from "./group_remove_button.js";
import "../styles/tab_bar.css";
import "../styles/tab_bar_dropdown.css";

/** Vintage folder icon SVG using theme CSS variables */
function FolderIcon(): React.JSX.Element {
  return (
    <svg
      className="tab-folder-icon"
      width="18"
      height="18"
      viewBox="0 0 96 96"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      {/* Folder tab */}
      <path
        className="folder-tab"
        d="M18 26 C18 22.7 20.7 20 24 20 H40 C42 20 43.5 20.7 44.9 22.2 L50.2 27.2 C51.1 28.1 52.2 28.6 53.5 28.6 H72 C75.3 28.6 78 31.3 78 34.6 V38 H18 Z"
        strokeWidth="3"
        strokeLinejoin="round"
      />
      {/* Folder body */}
      <path
        className="folder-body"
        d="M16 38 H80 C83 38 85.5 40.5 85.5 43.5 V70 C85.5 76.4 80.4 81.5 74 81.5 H22 C15.6 81.5 10.5 76.4 10.5 70 V43.5 C10.5 40.5 13 38 16 38 Z"
        strokeWidth="3"
        strokeLinejoin="round"
      />
    </svg>
  );
}

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
  const [openDropdownId, setOpenDropdownId] = useState<string | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const {
    isAdding,
    addError,
    addWorktreeToGroup,
    addWorktreeToSingle,
  } = useWorktreeAdd(
    onRepoAdded ? { onRepoChange, onRepoAdded } : { onRepoChange }
  );

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

  const handleAddWorktreeToGroup = useCallback(
    async (groupId: string, groupLabel: string) => {
      const didAdd = await addWorktreeToGroup(groupId, groupLabel);
      if (didAdd) {
        setOpenDropdownId(null);
      }
    },
    [addWorktreeToGroup]
  );

  const handleAddWorktreeToSingle = useCallback(
    async (singleTab: SingleTab) => {
      const didAdd = await addWorktreeToSingle(singleTab);
      if (didAdd) {
        setOpenDropdownId(null);
      }
    },
    [addWorktreeToSingle]
  );

  const handleOpenFolder = useCallback(async (wslPath: string) => {
    try {
      const windowsPath = await invoke<string>("convert_wsl_to_windows", { wslPath });
      await invoke("open_in_file_manager", { folderPath: windowsPath });
    } catch (err) {
      console.error("[TabBar] Failed to open folder:", err);
    }
  }, []);

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
              {isActive && (
                <button
                  type="button"
                  className="tab-folder-button"
                  onClick={() => { void handleOpenFolder(tab.wslPath); }}
                  title="Open folder in Explorer"
                  aria-label="Open repository folder"
                >
                  <FolderIcon />
                </button>
              )}
              <button
                className={`tab-button ${isActive ? "active" : ""}`}
                onClick={() => { onRepoChange(tab.repoId); }}
                type="button"
                aria-current={isActive ? "page" : undefined}
              >
                <span className="tab-label">{tab.label}</span>
              </button>
              {isActive && (
                <button
                  className="group-chevron"
                  onClick={(e) => { toggleDropdown(e, tab.repoId); }}
                  type="button"
                  aria-expanded={isOpen}
                  aria-label={`Add subfolder to ${tab.label}`}
                  title={`Add subfolder to ${tab.label}`}
                >
                  {isOpen ? "▲" : "▼"}
                </button>
              )}
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
                    + Add subfolder
                  </button>
                </div>
              )}
            </div>
          );
        } else {
          const isActive = isGroupActive(tab, activeRepoId);
          const activeLabel = getActiveRepoLabel(tab, activeRepoId);
          const isOpen = openDropdownId === tab.groupId;
          const repoCount = tab.repos.length;

          return (
            <div
              key={tab.groupId}
              className="group-tab-container"
              ref={isOpen ? dropdownRef : undefined}
            >
              {isActive && (
                <button
                  type="button"
                  className="tab-folder-button"
                  onClick={() => {
                    const activeRepo = tab.repos.find((r) => r.repoId === activeRepoId);
                    if (activeRepo) void handleOpenFolder(activeRepo.wslPath);
                  }}
                  title="Open folder in Explorer"
                  aria-label="Open repository folder"
                >
                  <FolderIcon />
                </button>
              )}
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
              {isActive && (
                <button
                  className="group-chevron"
                  onClick={(e) => { toggleDropdown(e, tab.groupId); }}
                  type="button"
                  aria-expanded={isOpen}
                  aria-label={`Add subfolder to ${tab.groupLabel}`}
                  title={`Add subfolder to ${tab.groupLabel}`}
                >
                  {isOpen ? "▲" : "▼"}
                </button>
              )}
              <GroupRemoveButton
                groupId={tab.groupId}
                groupLabel={tab.groupLabel}
                repoCount={repoCount}
                variant="icon"
              />

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
                    + Add subfolder
                  </button>
                  <GroupRemoveButton
                    groupId={tab.groupId}
                    groupLabel={tab.groupLabel}
                    repoCount={repoCount}
                    variant="dropdown"
                    onRemoved={() => {
                      setOpenDropdownId(null);
                    }}
                  />
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

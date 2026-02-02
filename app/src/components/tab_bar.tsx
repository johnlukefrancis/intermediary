// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation with grouped repo dropdown support

import type React from "react";
import { useState, useCallback, useEffect, useRef } from "react";
import type { TabItem, SingleTab, GroupTab } from "../app.js";
import { useWorktreeAdd } from "../hooks/use_worktree_add.js";
import { AddRepoButton } from "./add_repo_button.js";
import { TabRemoveButton } from "./tab_remove_button.js";
import "../styles/tab_bar.css";
import "../styles/tab_bar_dropdown.css";

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
              {isActive && (
                <button
                  className="group-chevron"
                  onClick={(e) => { toggleDropdown(e, tab.repoId); }}
                  type="button"
                  aria-expanded={isOpen}
                  aria-label={`Add folder to ${tab.label}`}
                  title={`Add folder to ${tab.label}`}
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
                    + Add folder
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
              {isActive && (
                <button
                  className="group-chevron"
                  onClick={(e) => { toggleDropdown(e, tab.groupId); }}
                  type="button"
                  aria-expanded={isOpen}
                  aria-label={`Select ${tab.groupLabel} folder`}
                  title={`Select ${tab.groupLabel} folder`}
                >
                  {isOpen ? "▲" : "▼"}
                </button>
              )}

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
                    + Add folder
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

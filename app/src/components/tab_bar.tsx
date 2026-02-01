// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation with grouped repo dropdown support

import type React from "react";
import { useState, useCallback, useEffect, useRef } from "react";
import type { TabItem, SingleTab, GroupTab } from "../app.js";
import { AddRepoButton } from "./add_repo_button.js";
import { TabRemoveButton } from "./tab_remove_button.js";
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
  const [openGroupId, setOpenGroupId] = useState<string | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown on outside click
  useEffect(() => {
    if (!openGroupId) return;

    function handleClickOutside(e: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setOpenGroupId(null);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [openGroupId]);

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
    (e: React.MouseEvent, groupId: string) => {
      e.stopPropagation();
      setOpenGroupId((prev) => (prev === groupId ? null : groupId));
    },
    []
  );

  const handleRepoSelect = useCallback(
    (repoId: string) => {
      onRepoChange(repoId);
      setOpenGroupId(null);
    },
    [onRepoChange]
  );

  return (
    <nav className="tab-bar">
      {tabs.map((tab) => {
        if (tab.type === "single") {
          return (
            <div key={tab.repoId} className="single-tab-container">
              <SingleTabButton
                tab={tab}
                isActive={activeRepoId === tab.repoId}
                onClick={() => { onRepoChange(tab.repoId); }}
              />
              <TabRemoveButton
                repoId={tab.repoId}
                label={tab.label}
              />
            </div>
          );
        } else {
          const isActive = isGroupActive(tab, activeRepoId);
          const activeLabel = getActiveRepoLabel(tab, activeRepoId);
          const isOpen = openGroupId === tab.groupId;

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

interface SingleTabButtonProps {
  tab: SingleTab;
  isActive: boolean;
  onClick: () => void;
}

function SingleTabButton({ tab, isActive, onClick }: SingleTabButtonProps): React.JSX.Element {
  return (
    <button
      className={`tab-button ${isActive ? "active" : ""}`}
      onClick={onClick}
      type="button"
      aria-current={isActive ? "page" : undefined}
    >
      <span className="tab-label">{tab.label}</span>
    </button>
  );
}

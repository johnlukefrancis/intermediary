// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation with grouped repo dropdown support

import type React from "react";
import { useState, useCallback, useEffect, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TabItem, SingleTab, GroupTab } from "../app.js";
import type { RepoRoot, TabTheme } from "../shared/config.js";
import { useWorktreeAdd } from "../hooks/use_worktree_add.js";
import { AddRepoButton } from "./add_repo_button.js";
import {
  GroupTabItem,
  SingleTabItem,
  isGroupActive,
} from "./tab_bar/tab_bar_items.js";
import {
  DEFAULT_ACCENT_HEX,
  hexToAccentCssVars,
} from "../lib/theme/accent_utils.js";
import "../styles/tab_bar.css";
import "../styles/tab_bar_dropdown.css";

interface TabBarProps {
  tabs: TabItem[];
  activeRepoId: string | null;
  tabThemes: Record<string, TabTheme>;
  lastActiveGroupRepoIds: Record<string, string>;
  onRepoChange: (repoId: string) => void;
  onRepoAdded?: (repoId: string) => void;
}

function getThemeKey(tab: TabItem): string {
  return tab.type === "group" ? tab.groupId : tab.repoId;
}

export function TabBar({
  tabs,
  activeRepoId,
  tabThemes,
  lastActiveGroupRepoIds,
  onRepoChange,
  onRepoAdded,
}: TabBarProps): React.JSX.Element {
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
  const defaultAccentStyle = useMemo(
    () => hexToAccentCssVars(DEFAULT_ACCENT_HEX) as React.CSSProperties,
    []
  );
  const tabAccentStyles = useMemo(() => {
    const styles = new Map<string, React.CSSProperties>();
    for (const tab of tabs) {
      const key = getThemeKey(tab);
      const accentHex = tabThemes[key]?.accentHex ?? DEFAULT_ACCENT_HEX;
      styles.set(key, hexToAccentCssVars(accentHex) as React.CSSProperties);
    }
    return styles;
  }, [tabs, tabThemes]);

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
      if (isGroupActive(group, activeRepoId)) return;
      const lastActiveId = lastActiveGroupRepoIds[group.groupId];
      const targetRepo =
        group.repos.find((repo) => repo.repoId === lastActiveId) ?? group.repos[0];
      if (targetRepo) {
        onRepoChange(targetRepo.repoId);
      }
    },
    [activeRepoId, lastActiveGroupRepoIds, onRepoChange]
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

  const handleOpenFolder = useCallback(async (root: RepoRoot) => {
    try {
      await invoke("open_in_file_manager", { folderPath: root.path });
    } catch (err) {
      console.error("[TabBar] Failed to open folder:", err);
    }
  }, []);

  return (
    <nav className="tab-bar">
      {tabs.map((tab) => {
        const isOpen = openDropdownId === (tab.type === "group" ? tab.groupId : tab.repoId);
        const themeKey = getThemeKey(tab);
        const accentStyle = tabAccentStyles.get(themeKey) ?? defaultAccentStyle;

        if (tab.type === "single") {
          return (
            <SingleTabItem
              key={tab.repoId}
              tab={tab}
              isActive={activeRepoId === tab.repoId}
              isOpen={isOpen}
              isAdding={isAdding}
              addError={addError}
              accentStyle={accentStyle}
              dropdownRef={dropdownRef}
              onRepoChange={onRepoChange}
              onOpenFolder={(root) => {
                void handleOpenFolder(root);
              }}
              onToggleDropdown={toggleDropdown}
              onAddWorktree={(singleTab) => {
                void handleAddWorktreeToSingle(singleTab);
              }}
            />
          );
        }

        return (
          <GroupTabItem
            key={tab.groupId}
            tab={tab}
            activeRepoId={activeRepoId}
            isOpen={isOpen}
            isAdding={isAdding}
            addError={addError}
            accentStyle={accentStyle}
            dropdownRef={dropdownRef}
            lastActiveGroupRepoIds={lastActiveGroupRepoIds}
            onGroupClick={handleGroupClick}
            onRepoSelect={handleRepoSelect}
            onOpenFolder={(root) => {
              void handleOpenFolder(root);
            }}
            onToggleDropdown={toggleDropdown}
            onAddWorktreeToGroup={(groupId, groupLabel) => {
              void handleAddWorktreeToGroup(groupId, groupLabel);
            }}
            onCloseDropdown={() => {
              setOpenDropdownId(null);
            }}
          />
        );
      })}
      <AddRepoButton {...(onRepoAdded ? { onRepoAdded } : {})} />
    </nav>
  );
}

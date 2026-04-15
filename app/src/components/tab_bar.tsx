// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation with grouped repo dropdown support and scroll overflow arrows

import type React from "react";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TabItem, SingleTab, GroupTab } from "../app.js";
import type { RepoRoot, TabTheme } from "../shared/config.js";
import { useConfig } from "../hooks/use_config.js";
import { useWorktreeAdd } from "../hooks/use_worktree_add.js";
import { useTabBarDropdown } from "../hooks/use_tab_bar_dropdown.js";
import { useTabBarScroll } from "../hooks/use_tab_bar_scroll.js";
import { AddRepoButton } from "./add_repo_button.js";
import {
  GroupTabItem,
  SingleTabItem,
  isGroupActive,
} from "./tab_bar/tab_bar_items.js";
import {
  GroupTabDropdown,
  SingleTabDropdown,
} from "./tab_bar/tab_bar_dropdowns.js";
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

function getDropdownId(tab: TabItem): string {
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
  const {
    config: { agentDistro },
  } = useConfig();
  const navRef = useRef<HTMLElement>(null);
  const {
    trackRef,
    scrollState,
    scrollLeft,
    scrollRight,
  } = useTabBarScroll();
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
  const {
    closeDropdown,
    dropdownAnchorStyle,
    dropdownRef,
    openDropdownId,
    registerDropdownTrigger,
    toggleDropdown,
  } = useTabBarDropdown({ navRef, trackRef });
  const tabAccentStyles = useMemo(() => {
    const styles = new Map<string, React.CSSProperties>();
    for (const tab of tabs) {
      const key = getThemeKey(tab);
      const accentHex = tabThemes[key]?.accentHex ?? DEFAULT_ACCENT_HEX;
      styles.set(key, hexToAccentCssVars(accentHex) as React.CSSProperties);
    }
    return styles;
  }, [tabs, tabThemes]);

  /* Auto-scroll active tab into view */
  useEffect(() => {
    if (!activeRepoId || !trackRef.current) return;
    const track = trackRef.current;
    const activeEl = track.querySelector<HTMLElement>("[aria-current='page']");
    if (!activeEl) return;

    const trackRect = track.getBoundingClientRect();
    const elRect = activeEl.getBoundingClientRect();

    if (elRect.left < trackRect.left) {
      track.scrollBy({ left: elRect.left - trackRect.left - 8, behavior: "smooth" });
    } else if (elRect.right > trackRect.right) {
      track.scrollBy({ left: elRect.right - trackRect.right + 8, behavior: "smooth" });
    }
  }, [activeRepoId, trackRef]);

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

  const handleRepoSelect = useCallback(
    (repoId: string) => {
      onRepoChange(repoId);
      closeDropdown();
    },
    [closeDropdown, onRepoChange]
  );

  const handleAddWorktreeToGroup = useCallback(
    async (groupId: string, groupLabel: string) => {
      const didAdd = await addWorktreeToGroup(groupId, groupLabel);
      if (didAdd) {
        closeDropdown();
      }
    },
    [addWorktreeToGroup, closeDropdown]
  );

  const handleAddWorktreeToSingle = useCallback(
    async (singleTab: SingleTab) => {
      const didAdd = await addWorktreeToSingle(singleTab);
      if (didAdd) {
        closeDropdown();
      }
    },
    [addWorktreeToSingle, closeDropdown]
  );

  const handleOpenFolder = useCallback(async (root: RepoRoot) => {
    try {
      await invoke("open_in_file_manager", {
        folderPath: root.path,
        distroOverride: agentDistro,
      });
    } catch (err) {
      console.error("[TabBar] Failed to open folder:", err);
    }
  }, [agentDistro]);

  /* Find open dropdown's tab + accent for rendering outside the track */
  const openTab = openDropdownId
    ? tabs.find((t) => getDropdownId(t) === openDropdownId)
    : null;
  const openTabAccent = openTab
    ? (tabAccentStyles.get(getThemeKey(openTab)) ?? defaultAccentStyle)
    : defaultAccentStyle;

  return (
    <nav className="tab-bar" ref={navRef}>
      <button
        type="button"
        className={`tab-bar-scroll-arrow left${
          !scrollState.isOverflowing ? " gone" : !scrollState.canScrollLeft ? " hidden" : ""
        }`}
        onClick={scrollLeft}
        aria-label="Scroll tabs left"
      >
        ◀
      </button>

      <div className="tab-bar-track" ref={trackRef}>
        {tabs.map((tab) => {
          const isOpen = openDropdownId === getDropdownId(tab);
          const themeKey = getThemeKey(tab);
          const accentStyle = tabAccentStyles.get(themeKey) ?? defaultAccentStyle;

          if (tab.type === "single") {
            return (
              <SingleTabItem
                key={tab.repoId}
                tab={tab}
                isActive={activeRepoId === tab.repoId}
                isOpen={isOpen}
                accentStyle={accentStyle}
                onRepoChange={onRepoChange}
                onOpenFolder={(root) => {
                  void handleOpenFolder(root);
                }}
                onRegisterDropdownTrigger={registerDropdownTrigger}
                onToggleDropdown={toggleDropdown}
              />
            );
          }

          return (
            <GroupTabItem
              key={tab.groupId}
              tab={tab}
              activeRepoId={activeRepoId}
              isOpen={isOpen}
              accentStyle={accentStyle}
              lastActiveGroupRepoIds={lastActiveGroupRepoIds}
              onGroupClick={handleGroupClick}
              onOpenFolder={(root) => {
                void handleOpenFolder(root);
              }}
              onRegisterDropdownTrigger={registerDropdownTrigger}
              onToggleDropdown={toggleDropdown}
            />
          );
        })}
      </div>

      <button
        type="button"
        className={`tab-bar-scroll-arrow right${
          !scrollState.isOverflowing ? " gone" : !scrollState.canScrollRight ? " hidden" : ""
        }`}
        onClick={scrollRight}
        aria-label="Scroll tabs right"
      >
        ▶
      </button>

      <AddRepoButton {...(onRepoAdded ? { onRepoAdded } : {})} />

      {/* Dropdown rendered outside track to avoid overflow clip */}
      {openTab && (
        <div
          className="tab-bar-dropdown-anchor"
          ref={dropdownRef}
          style={dropdownAnchorStyle ?? undefined}
        >
          {openTab.type === "single" ? (
            <SingleTabDropdown
              tab={openTab}
              isAdding={isAdding}
              addError={addError}
              accentStyle={openTabAccent}
              onAddWorktree={(singleTab) => {
                void handleAddWorktreeToSingle(singleTab);
              }}
            />
          ) : (
            <GroupTabDropdown
              tab={openTab}
              activeRepoId={activeRepoId}
              isAdding={isAdding}
              addError={addError}
              accentStyle={openTabAccent}
              onRepoSelect={handleRepoSelect}
              onAddWorktreeToGroup={(groupId, groupLabel) => {
                void handleAddWorktreeToGroup(groupId, groupLabel);
              }}
              onCloseDropdown={() => {
                closeDropdown();
              }}
            />
          )}
        </div>
      )}
    </nav>
  );
}

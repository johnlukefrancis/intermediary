// Path: app/src/components/tab_bar/tab_bar_items.tsx
// Description: Focused tab item renderers for single and grouped repository tabs

import type React from "react";
import type { GroupTab, SingleTab } from "../../app.js";
import type { RepoRoot } from "../../shared/config.js";
import { GroupRemoveButton } from "../group_remove_button.js";
import { TabRemoveButton } from "../tab_remove_button.js";

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
      <path
        className="folder-tab"
        d="M18 26 C18 22.7 20.7 20 24 20 H40 C42 20 43.5 20.7 44.9 22.2 L50.2 27.2 C51.1 28.1 52.2 28.6 53.5 28.6 H72 C75.3 28.6 78 31.3 78 34.6 V38 H18 Z"
        strokeWidth="3"
        strokeLinejoin="round"
      />
      <path
        className="folder-body"
        d="M16 38 H80 C83 38 85.5 40.5 85.5 43.5 V70 C85.5 76.4 80.4 81.5 74 81.5 H22 C15.6 81.5 10.5 76.4 10.5 70 V43.5 C10.5 40.5 13 38 16 38 Z"
        strokeWidth="3"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function isGroupActive(
  group: GroupTab,
  activeRepoId: string | null
): boolean {
  return group.repos.some((r) => r.repoId === activeRepoId);
}

function getActiveRepoLabel(
  group: GroupTab,
  activeRepoId: string | null,
  lastActiveGroupRepoIds: Record<string, string>
): string {
  const activeRepo = group.repos.find((r) => r.repoId === activeRepoId);
  if (activeRepo) return activeRepo.label;
  const lastActiveId = lastActiveGroupRepoIds[group.groupId];
  const lastActiveRepo = group.repos.find((r) => r.repoId === lastActiveId);
  return lastActiveRepo?.label ?? group.repos[0]?.label ?? "";
}

interface SingleTabItemProps {
  tab: SingleTab;
  isActive: boolean;
  isOpen: boolean;
  isAdding: boolean;
  addError: string | null;
  accentStyle: React.CSSProperties;
  dropdownRef: React.Ref<HTMLDivElement>;
  onRepoChange: (repoId: string) => void;
  onOpenFolder: (root: RepoRoot) => void;
  onToggleDropdown: (e: React.MouseEvent, dropdownId: string) => void;
  onAddWorktree: (tab: SingleTab) => void;
}

export function SingleTabItem({
  tab,
  isActive,
  isOpen,
  isAdding,
  addError,
  accentStyle,
  dropdownRef,
  onRepoChange,
  onOpenFolder,
  onToggleDropdown,
  onAddWorktree,
}: SingleTabItemProps): React.JSX.Element {
  return (
    <div
      key={tab.repoId}
      className="single-tab-container"
      style={accentStyle}
      ref={isOpen ? dropdownRef : undefined}
    >
      {isActive && (
        <button
          type="button"
          className="tab-folder-button"
          onClick={() => {
            onOpenFolder(tab.root);
          }}
          title="Open folder in Explorer"
          aria-label="Open repository folder"
        >
          <FolderIcon />
        </button>
      )}
      <button
        className={`tab-button ${isActive ? "active" : ""}`}
        onClick={() => {
          onRepoChange(tab.repoId);
        }}
        type="button"
        aria-current={isActive ? "page" : undefined}
      >
        <span className="tab-label">{tab.label}</span>
      </button>
      {isActive && (
        <button
          className="group-chevron"
          onClick={(e) => {
            onToggleDropdown(e, tab.repoId);
          }}
          type="button"
          aria-expanded={isOpen}
          aria-label={`Add subfolder to ${tab.label}`}
          title={`Add subfolder to ${tab.label}`}
        >
          {isOpen ? "▲" : "▼"}
        </button>
      )}
      <TabRemoveButton repoId={tab.repoId} label={tab.label} />

      {isOpen && (
        <div className="group-dropdown">
          {addError && <div className="group-dropdown-error">{addError}</div>}
          <button
            className="group-dropdown-add"
            onClick={() => {
              onAddWorktree(tab);
            }}
            disabled={isAdding}
            type="button"
          >
            + Add subfolder
          </button>
        </div>
      )}
    </div>
  );
}

interface GroupTabItemProps {
  tab: GroupTab;
  activeRepoId: string | null;
  isOpen: boolean;
  isAdding: boolean;
  addError: string | null;
  accentStyle: React.CSSProperties;
  dropdownRef: React.Ref<HTMLDivElement>;
  lastActiveGroupRepoIds: Record<string, string>;
  onGroupClick: (group: GroupTab) => void;
  onRepoSelect: (repoId: string) => void;
  onOpenFolder: (root: RepoRoot) => void;
  onToggleDropdown: (e: React.MouseEvent, dropdownId: string) => void;
  onAddWorktreeToGroup: (groupId: string, groupLabel: string) => void;
  onCloseDropdown: () => void;
}

export function GroupTabItem({
  tab,
  activeRepoId,
  isOpen,
  isAdding,
  addError,
  accentStyle,
  dropdownRef,
  lastActiveGroupRepoIds,
  onGroupClick,
  onRepoSelect,
  onOpenFolder,
  onToggleDropdown,
  onAddWorktreeToGroup,
  onCloseDropdown,
}: GroupTabItemProps): React.JSX.Element {
  const isActive = isGroupActive(tab, activeRepoId);
  const activeLabel = getActiveRepoLabel(tab, activeRepoId, lastActiveGroupRepoIds);
  const repoCount = tab.repos.length;
  const trimmedGroupLabel = tab.groupLabel.trim();
  const trimmedActiveLabel = activeLabel.trim();
  const showActiveSuffix =
    trimmedActiveLabel.length > 0 && trimmedActiveLabel !== trimmedGroupLabel;
  const groupDisplayLabel = trimmedGroupLabel || tab.groupId;
  const fullGroupLabel = showActiveSuffix
    ? `${groupDisplayLabel}: ${trimmedActiveLabel}`
    : groupDisplayLabel;

  return (
    <div
      key={tab.groupId}
      className="group-tab-container"
      style={accentStyle}
      ref={isOpen ? dropdownRef : undefined}
    >
      {isActive && (
        <button
          type="button"
          className="tab-folder-button"
          onClick={() => {
            const activeRepo = tab.repos.find((r) => r.repoId === activeRepoId);
            if (activeRepo) onOpenFolder(activeRepo.root);
          }}
          title="Open folder in Explorer"
          aria-label="Open repository folder"
        >
          <FolderIcon />
        </button>
      )}
      <button
        className={`tab-button ${isActive ? "active" : ""}`}
        onClick={() => {
          onGroupClick(tab);
        }}
        type="button"
        aria-current={isActive ? "page" : undefined}
      >
        <span className="tab-label">{fullGroupLabel}</span>
      </button>
      {isActive && (
        <button
          className="group-chevron"
          onClick={(e) => {
            onToggleDropdown(e, tab.groupId);
          }}
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
                  onClick={() => {
                    onRepoSelect(repo.repoId);
                  }}
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
          {addError && <div className="group-dropdown-error">{addError}</div>}
          <button
            className="group-dropdown-add"
            onClick={() => {
              onAddWorktreeToGroup(tab.groupId, tab.groupLabel);
            }}
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
            onRemoved={onCloseDropdown}
          />
        </div>
      )}
    </div>
  );
}

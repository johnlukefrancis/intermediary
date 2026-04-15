// Path: app/src/components/tab_bar/tab_bar_dropdowns.tsx
// Description: Dropdown panels for single-repo and grouped-repo tab-bar actions

import type React from "react";
import type { GroupTab, SingleTab } from "../../app.js";
import { GroupRemoveButton } from "../group_remove_button.js";
import { TabRemoveButton } from "../tab_remove_button.js";

interface SingleTabDropdownProps {
  tab: SingleTab;
  isAdding: boolean;
  addError: string | null;
  accentStyle: React.CSSProperties;
  onAddWorktree: (tab: SingleTab) => void;
}

export function SingleTabDropdown({
  tab,
  isAdding,
  addError,
  accentStyle,
  onAddWorktree,
}: SingleTabDropdownProps): React.JSX.Element {
  return (
    <div className="group-dropdown" style={accentStyle}>
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
  );
}

interface GroupTabDropdownProps {
  tab: GroupTab;
  activeRepoId: string | null;
  isAdding: boolean;
  addError: string | null;
  accentStyle: React.CSSProperties;
  onRepoSelect: (repoId: string) => void;
  onAddWorktreeToGroup: (groupId: string, groupLabel: string) => void;
  onCloseDropdown: () => void;
}

export function GroupTabDropdown({
  tab,
  activeRepoId,
  isAdding,
  addError,
  accentStyle,
  onRepoSelect,
  onAddWorktreeToGroup,
  onCloseDropdown,
}: GroupTabDropdownProps): React.JSX.Element {
  const repoCount = tab.repos.length;

  return (
    <div className="group-dropdown" style={accentStyle}>
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
  );
}

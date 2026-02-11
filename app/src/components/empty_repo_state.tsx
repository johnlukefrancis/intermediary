// Path: app/src/components/empty_repo_state.tsx
// Description: Empty state UI when no repos are configured

import type React from "react";
import { AddRepoButton } from "./add_repo_button.js";
import "../styles/empty_repo_state.css";

interface EmptyRepoStateProps {
  onRepoAdded?: (repoId: string) => void;
  persistenceLocked?: boolean;
  persistenceLockReason?: string | null;
}

export function EmptyRepoState({
  onRepoAdded,
  persistenceLocked = false,
  persistenceLockReason = null,
}: EmptyRepoStateProps): React.JSX.Element {
  if (persistenceLocked) {
    return (
      <div className="empty-repo-state">
        <div className="empty-repo-state__panel glass-surface">
          <div className="empty-repo-state__content">
            <p className="empty-repo-state__title">Configuration load failed</p>
            <p className="empty-repo-state__subtitle">
              Config persistence is locked for this session to prevent overwriting your saved
              repositories.
            </p>
            {persistenceLockReason ? (
              <p className="empty-repo-state__subtitle">
                Details: {persistenceLockReason}
              </p>
            ) : null}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="empty-repo-state">
      <div className="empty-repo-state__panel glass-surface">
        <div className="empty-repo-state__content">
          <p className="empty-repo-state__title">No repositories configured</p>
          <p className="empty-repo-state__subtitle">
            Add a repository to begin.
          </p>
          <AddRepoButton
            {...(onRepoAdded ? { onRepoAdded } : {})}
            className="empty-repo-state__button"
          />
        </div>
      </div>
    </div>
  );
}

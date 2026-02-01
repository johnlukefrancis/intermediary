// Path: app/src/components/empty_repo_state.tsx
// Description: Empty state UI when no repos are configured

import type React from "react";
import { AddRepoButton } from "./add_repo_button.js";
import "../styles/empty_repo_state.css";

interface EmptyRepoStateProps {
  onRepoAdded?: (repoId: string) => void;
}

export function EmptyRepoState({
  onRepoAdded,
}: EmptyRepoStateProps): React.JSX.Element {
  return (
    <div className="empty-repo-state">
      <div className="empty-repo-state__content">
        <p className="empty-repo-state__title">No repositories configured</p>
        <p className="empty-repo-state__subtitle">
          Add a repository to start bundling context for your AI agents.
        </p>
        <AddRepoButton
          {...(onRepoAdded ? { onRepoAdded } : {})}
          className="empty-repo-state__button"
        />
      </div>
    </div>
  );
}

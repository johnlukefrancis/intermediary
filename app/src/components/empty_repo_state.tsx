// Path: app/src/components/empty_repo_state.tsx
// Description: Empty state UI when no repos are configured

import type React from "react";
import { AddRepoButton } from "./add_repo_button.js";
import { EmptyStatePanel } from "./empty_state_panel.js";

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
    return <EmptyStatePanel title="Configuration load failed"
      description="Config persistence is locked for this session to prevent overwriting your saved repositories."
      details={persistenceLockReason ? `Details: ${persistenceLockReason}` : null} />;
  }

  return <EmptyStatePanel title="No repositories configured" description="Add a repository to begin."
    action={<AddRepoButton {...(onRepoAdded ? { onRepoAdded } : {})} variant="empty_state" />} />;
}

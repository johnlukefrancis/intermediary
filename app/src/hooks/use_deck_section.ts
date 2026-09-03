// Path: app/src/hooks/use_deck_section.ts
// Description: One owner for the deck section: persisted right rail plus the handset-only FILES flag

import { useCallback, useState } from "react";
import type { ActiveRail } from "../shared/config.js";
import { useConfig } from "./use_config.js";

/** Handset sections add FILES in front of the two rail sections */
export type DeckSection = "files" | ActiveRail;

export interface DeckSectionState {
  /** Persisted rail section shown in the standard deck and workspace mode */
  activeRail: ActiveRail;
  setActiveRail: (rail: ActiveRail) => void;
  /** Handset section: FILES when the local flag is set, otherwise the persisted rail */
  handsetSection: DeckSection;
  setHandsetSection: (section: DeckSection) => void;
}

export function useDeckSection(): DeckSectionState {
  const { config, setActiveRail: persistActiveRail } = useConfig();
  const activeRail = config.uiState.activeRail;
  const [filesActive, setFilesActive] = useState(true);

  const setActiveRail = useCallback(
    (rail: ActiveRail) => {
      setFilesActive(false);
      persistActiveRail(rail);
    },
    [persistActiveRail]
  );

  const setHandsetSection = useCallback(
    (section: DeckSection) => {
      if (section === "files") {
        setFilesActive(true);
        return;
      }
      setActiveRail(section);
    },
    [setActiveRail]
  );

  return {
    activeRail,
    setActiveRail,
    handsetSection: filesActive ? "files" : activeRail,
    setHandsetSection,
  };
}

// Path: app/src/components/options/theme_section.tsx
// Description: Options panel theme controls (warm mode toggle + texture/accent per tab)

import type React from "react";
import { useMemo, useState, useRef, useEffect } from "react";
import type { TabTheme, ThemeMode } from "../../shared/config.js";
import { DEFAULT_ACCENT_HEX } from "../../lib/theme/accent_utils.js";
import {
  DEFAULT_TEXTURE_ID,
  getTextureOptions,
} from "../../lib/theme/texture_catalog.js";
import { TexturePicker } from "./texture_picker.js";
import {
  TriStateRocker,
  type TriStateOption,
} from "./controls/tri_state_rocker.js";
import { OptionsFieldRow } from "./layout/options_field_row.js";

interface ThemeEntry {
  tabKey: string;
  id: string;
  label: string;
  kind: "repo" | "group";
}

interface ThemeSectionProps {
  entries: ThemeEntry[];
  tabThemes: Record<string, TabTheme>;
  themeMode: ThemeMode;
  setThemeMode: (mode: ThemeMode) => void;
  setTabThemeAccent: (tabKey: string, accentHex: string) => void;
  setTabThemeTexture: (tabKey: string, textureId: string) => void;
  clearTabTheme: (tabKey: string) => void;
  renameRepoLabel: (repoId: string, label: string) => void;
  renameGroupLabel: (groupId: string, label: string) => void;
}

const THEME_MODES: ReadonlyArray<TriStateOption<ThemeMode>> = [
  { value: "dark", label: "DARK" },
  { value: "light", label: "LIGHT" },
  { value: "warm", label: "WARM" },
];

export function ThemeSection({
  entries,
  tabThemes,
  themeMode,
  setThemeMode,
  setTabThemeAccent,
  setTabThemeTexture,
  clearTabTheme,
  renameRepoLabel,
  renameGroupLabel,
}: ThemeSectionProps): React.JSX.Element {
  const textureOptions = useMemo(() => getTextureOptions(), []);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [draftLabel, setDraftLabel] = useState("");
  const inputRef = useRef<HTMLInputElement | null>(null);
  const cancelCommitRef = useRef(false);

  useEffect(() => {
    if (editingKey && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingKey]);

  function commitRename(entry: ThemeEntry): void {
    if (cancelCommitRef.current) {
      cancelCommitRef.current = false;
      setEditingKey(null);
      return;
    }
    const trimmed = draftLabel.trim();
    if (!trimmed) {
      setEditingKey(null);
      return;
    }
    if (trimmed !== entry.label) {
      if (entry.kind === "group") {
        renameGroupLabel(entry.id, trimmed);
      } else {
        renameRepoLabel(entry.id, trimmed);
      }
    }
    setEditingKey(null);
  }

  return (
    <div className="options-section">
      <OptionsFieldRow
        label="Theme"
        title="Global color scheme"
        controlAlign="stretch"
        control={(
          <TriStateRocker
            value={themeMode}
            options={THEME_MODES}
            onChange={setThemeMode}
            ariaLabel="Theme mode"
            className="tri-state-rocker--theme"
          />
        )}
      />
      {entries.length === 0 ? null : (
        <div className="options-theme-list">
          {entries.map((entry) => {
            const theme = tabThemes[entry.tabKey];
            const currentHex = theme?.accentHex ?? DEFAULT_ACCENT_HEX;
            const currentTexture = theme?.textureId ?? DEFAULT_TEXTURE_ID;
            const resolvedTexture =
              textureOptions.find((option) => option.id === currentTexture) ??
              textureOptions.find((option) => option.id === DEFAULT_TEXTURE_ID) ??
              textureOptions[0];
            const selectedTextureId = resolvedTexture?.id ?? DEFAULT_TEXTURE_ID;
            const hasCustomTheme = entry.tabKey in tabThemes;
            const isEditing = editingKey === entry.tabKey;

            return (
              <div key={entry.tabKey} className="options-theme-row">
                <div className="options-theme-label-row">
                  {isEditing ? (
                    <input
                      ref={inputRef}
                      type="text"
                      value={draftLabel}
                      onChange={(event) => {
                        setDraftLabel(event.target.value);
                      }}
                      onBlur={() => {
                        commitRename(entry);
                      }}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") {
                          event.preventDefault();
                          commitRename(entry);
                        }
                        if (event.key === "Escape") {
                          event.preventDefault();
                          cancelCommitRef.current = true;
                          setEditingKey(null);
                          setDraftLabel("");
                        }
                      }}
                      className="options-theme-input"
                      aria-label={`Rename ${entry.label}`}
                    />
                  ) : (
                    <span className="options-theme-label" title={entry.label}>
                      {entry.label}
                    </span>
                  )}
                  <button
                    type="button"
                    className="options-theme-rename"
                    onClick={() => {
                      setEditingKey(entry.tabKey);
                      setDraftLabel(entry.label);
                    }}
                    aria-label={`Rename ${entry.label}`}
                    title={`Rename ${entry.label}`}
                  >
                    <svg
                      className="options-theme-rename-icon"
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                      xmlns="http://www.w3.org/2000/svg"
                      aria-hidden="true"
                    >
                      <path
                        d="M4 20h4l10-10-4-4L4 16v4Z"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinejoin="round"
                      />
                      <path
                        d="m14 6 4 4"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                      />
                    </svg>
                  </button>
                </div>
                <div className="options-theme-controls">
                  <TexturePicker
                    options={textureOptions}
                    selectedId={selectedTextureId}
                    onSelect={(textureId) => {
                      setTabThemeTexture(entry.tabKey, textureId);
                    }}
                  />
                  <input
                    type="color"
                    value={currentHex}
                    onChange={(e) => {
                      setTabThemeAccent(entry.tabKey, e.target.value);
                    }}
                    className="options-color-input"
                    title="Choose accent color"
                  />
                  {hasCustomTheme && (
                    <button
                      type="button"
                      className="options-reset-button"
                      onClick={() => {
                        clearTabTheme(entry.tabKey);
                      }}
                      title="Reset to default"
                    >
                      ×
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

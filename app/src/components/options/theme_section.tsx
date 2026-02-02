// Path: app/src/components/options/theme_section.tsx
// Description: Options panel theme controls (texture + accent per tab)

import type React from "react";
import { useMemo } from "react";
import type { TabTheme } from "../../shared/config.js";
import { DEFAULT_ACCENT_HEX } from "../../lib/theme/accent_utils.js";
import {
  DEFAULT_TEXTURE_ID,
  getTextureOptions,
} from "../../lib/theme/texture_catalog.js";
import { TexturePicker } from "./texture_picker.js";

interface ThemeEntry {
  tabKey: string;
  label: string;
}

interface ThemeSectionProps {
  entries: ThemeEntry[];
  tabThemes: Record<string, TabTheme>;
  setTabThemeAccent: (tabKey: string, accentHex: string) => void;
  setTabThemeTexture: (tabKey: string, textureId: string) => void;
  clearTabTheme: (tabKey: string) => void;
}

export function ThemeSection({
  entries,
  tabThemes,
  setTabThemeAccent,
  setTabThemeTexture,
  clearTabTheme,
}: ThemeSectionProps): React.JSX.Element | null {
  const textureOptions = useMemo(() => getTextureOptions(), []);

  if (entries.length === 0) {
    return null;
  }

  return (
    <div className="options-section">
      <div className="options-section-title">Theme</div>
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

          return (
            <div key={entry.tabKey} className="options-theme-row">
              <span className="options-theme-label" title={entry.label}>
                {entry.label}
              </span>
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
    </div>
  );
}

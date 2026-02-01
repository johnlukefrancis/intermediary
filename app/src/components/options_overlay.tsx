// Path: app/src/components/options_overlay.tsx
// Description: Full-screen transparent overlay with options panel for app settings

import type React from "react";
import { useCallback, useState, useMemo, useEffect } from "react";
import { createPortal } from "react-dom";
import type { GlobalExcludes } from "../shared/config.js";
import "../styles/options_overlay.css";

interface OptionsOverlayProps {
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  stagingPath: string | null;
  globalExcludes: GlobalExcludes;
  setGlobalExcludes: (excludes: GlobalExcludes) => void;
  onClose: () => void;
}

/**
 * Parse comma-separated string to array of trimmed values
 */
function parseCommaSeparated(value: string): string[] {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function normalizeExtensions(values: string[]): string[] {
  return values
    .map((value) => value.trim())
    .filter((value) => value.length > 0)
    .map((value) => value.toLowerCase())
    .map((value) => (value.startsWith(".") ? value : `.${value}`));
}

function normalizePatterns(values: string[]): string[] {
  return values
    .map((value) => value.trim())
    .filter((value) => value.length > 0)
    .map((value) => value.replace(/^\/+|\/+$/g, ""))
    .filter((value) => value.length > 0)
    .map((value) => value.toLowerCase());
}

/**
 * Format array to comma-separated string
 */
function formatCommaSeparated(arr: string[]): string {
  return arr.join(", ");
}

export function OptionsOverlay({
  autoStageOnChange,
  setAutoStageOnChange,
  stagingPath,
  globalExcludes,
  setGlobalExcludes,
  onClose,
}: OptionsOverlayProps): React.JSX.Element {
  const [copyFeedback, setCopyFeedback] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const toggleAdvanced = useCallback(() => {
    setAdvancedOpen((prev) => !prev);
  }, []);

  // Local state for text inputs (commit on blur)
  const [extensionsText, setExtensionsText] = useState(() =>
    formatCommaSeparated(globalExcludes.extensions)
  );
  const [patternsText, setPatternsText] = useState(() =>
    formatCommaSeparated(globalExcludes.patterns)
  );

  useEffect(() => {
    setExtensionsText(formatCommaSeparated(globalExcludes.extensions));
    setPatternsText(formatCommaSeparated(globalExcludes.patterns));
  }, [globalExcludes.extensions, globalExcludes.patterns]);

  // Derive whether inputs are "dirty" (different from saved)
  const savedExtensionsText = useMemo(
    () => formatCommaSeparated(globalExcludes.extensions),
    [globalExcludes.extensions]
  );
  const savedPatternsText = useMemo(
    () => formatCommaSeparated(globalExcludes.patterns),
    [globalExcludes.patterns]
  );

  const handleCopyPath = useCallback(() => {
    if (!stagingPath) return;
    void navigator.clipboard.writeText(stagingPath).then(() => {
      setCopyFeedback(true);
      setTimeout(() => {
        setCopyFeedback(false);
      }, 1500);
    });
  }, [stagingPath]);

  const commitExcludes = useCallback(() => {
    const newExtensions = normalizeExtensions(parseCommaSeparated(extensionsText));
    const newPatterns = normalizePatterns(parseCommaSeparated(patternsText));
    // Only update if changed
    if (
      formatCommaSeparated(newExtensions) !== savedExtensionsText ||
      formatCommaSeparated(newPatterns) !== savedPatternsText
    ) {
      setGlobalExcludes({
        presets: globalExcludes.presets,
        extensions: newExtensions,
        patterns: newPatterns,
      });
    }
  }, [
    extensionsText,
    patternsText,
    savedExtensionsText,
    savedPatternsText,
    globalExcludes.presets,
    setGlobalExcludes,
  ]);

  const handleClose = useCallback(() => {
    commitExcludes();
    onClose();
  }, [commitExcludes, onClose]);

  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (event.target === event.currentTarget) {
        handleClose();
      }
    },
    [handleClose]
  );

  const setPresetValue = useCallback(
    (key: keyof GlobalExcludes["presets"], value: boolean) => {
      setGlobalExcludes({
        presets: {
          ...globalExcludes.presets,
          [key]: value,
        },
        extensions: globalExcludes.extensions,
        patterns: globalExcludes.patterns,
      });
    },
    [globalExcludes, setGlobalExcludes]
  );

  const allPresetsEnabled = useMemo(() => {
    const presets = globalExcludes.presets;
    return (
      presets.modelWeights &&
      presets.modelFormats &&
      presets.modelDirs &&
      presets.hfCaches &&
      presets.experimentLogs
    );
  }, [globalExcludes.presets]);

  const handleRecommendedToggle = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      const value = event.target.checked;
      setGlobalExcludes({
        presets: {
          modelWeights: value,
          modelFormats: value,
          modelDirs: value,
          hfCaches: value,
          experimentLogs: value,
        },
        extensions: globalExcludes.extensions,
        patterns: globalExcludes.patterns,
      });
    },
    [globalExcludes.extensions, globalExcludes.patterns, setGlobalExcludes]
  );

  const advancedSummary = useMemo(() => {
    const extCount = globalExcludes.extensions.length;
    const patternCount = globalExcludes.patterns.length;
    if (extCount === 0 && patternCount === 0) {
      return "Advanced";
    }
    const parts = [];
    if (extCount > 0) parts.push(`${extCount} ext`);
    if (patternCount > 0) parts.push(`${patternCount} path`);
    return `Advanced (${parts.join(", ")})`;
  }, [globalExcludes.extensions.length, globalExcludes.patterns.length]);

  return createPortal(
    <div className="options-overlay" onClick={handleBackdropClick}>
      <div className="options-panel">
        <button
          type="button"
          className="options-close-button"
          onClick={handleClose}
          aria-label="Close options"
        >
          ×
        </button>

        <div className="options-section">
          <div className="options-section-title">General</div>
          <div className="options-row">
            <span className="options-row-label">Auto-stage</span>
            <label className="vintage-toggle">
              <input
                type="checkbox"
                checked={autoStageOnChange}
                onChange={(event) => {
                  setAutoStageOnChange(event.target.checked);
                }}
              />
              <span className="vintage-toggle-track" aria-hidden="true" />
            </label>
          </div>
        </div>

        <div className="options-section">
          <div className="options-section-title">Recommended Excludes</div>
          <label className="options-checkbox-row">
            <input
              type="checkbox"
              checked={allPresetsEnabled}
              onChange={handleRecommendedToggle}
            />
            <span>
              Recommended (toggle all presets)
              <span className="options-hint">Safe defaults for large ML artifacts.</span>
            </span>
          </label>

          <label className="options-checkbox-row">
            <input
              type="checkbox"
              checked={globalExcludes.presets.modelWeights}
              onChange={(event) => {
                setPresetValue("modelWeights", event.target.checked);
              }}
            />
            <span>
              Model weights (.safetensors, .ckpt, .pt, .pth, .bin)
            </span>
          </label>

          <label className="options-checkbox-row">
            <input
              type="checkbox"
              checked={globalExcludes.presets.modelFormats}
              onChange={(event) => {
                setPresetValue("modelFormats", event.target.checked);
              }}
            />
            <span>
              Model formats (.onnx, .pb, .h5, .keras)
            </span>
          </label>

          <label className="options-checkbox-row">
            <input
              type="checkbox"
              checked={globalExcludes.presets.modelDirs}
              onChange={(event) => {
                setPresetValue("modelDirs", event.target.checked);
              }}
            />
            <span>
              Model directories (models/, checkpoints/, weights/)
            </span>
          </label>

          <label className="options-checkbox-row">
            <input
              type="checkbox"
              checked={globalExcludes.presets.hfCaches}
              onChange={(event) => {
                setPresetValue("hfCaches", event.target.checked);
              }}
            />
            <span>
              Hugging Face cache (.huggingface/, huggingface_hub/)
            </span>
          </label>

          <label className="options-checkbox-row">
            <input
              type="checkbox"
              checked={globalExcludes.presets.experimentLogs}
              onChange={(event) => {
                setPresetValue("experimentLogs", event.target.checked);
              }}
            />
            <span>
              Experiment logs (wandb/, mlruns/, lightning_logs/)
            </span>
          </label>
        </div>

        <div className="options-section">
          <button
            type="button"
            className="options-section-toggle"
            onClick={toggleAdvanced}
            aria-expanded={advancedOpen}
          >
            <span className="options-section-title">{advancedSummary}</span>
            <span className={`options-chevron ${advancedOpen ? "open" : ""}`}>
              ▸
            </span>
          </button>

          {advancedOpen && (
            <div className="options-section-content">
              <div className="options-row stacked">
                <span className="options-row-label">Extensions (comma-separated)</span>
                <input
                  type="text"
                  className="options-text-input"
                  value={extensionsText}
                  onChange={(e) => { setExtensionsText(e.target.value); }}
                  onBlur={commitExcludes}
                  placeholder=".safetensors, .ckpt, .pt"
                  title="Comma-separated file extensions to exclude from bundles"
                />
                <span className="options-hint">Case-insensitive; dot optional.</span>
              </div>

              <div className="options-row stacked">
                <span className="options-row-label">Path segments (comma-separated)</span>
                <input
                  type="text"
                  className="options-text-input"
                  value={patternsText}
                  onChange={(e) => { setPatternsText(e.target.value); }}
                  onBlur={commitExcludes}
                  placeholder="models, checkpoints, wandb"
                  title="Comma-separated path segments to exclude from bundles"
                />
                <span className="options-hint">Matches path segments only; no wildcards.</span>
              </div>
            </div>
          )}
        </div>

        {stagingPath && (
          <div className="options-section">
            <div className="options-section-title">Paths</div>
            <div className="options-row stacked">
              <span className="options-row-label">Staging Path</span>
              <button
                type="button"
                className={`path-chip ${copyFeedback ? "copied" : ""}`}
                onClick={handleCopyPath}
                title="Click to copy staging path"
              >
                {copyFeedback ? "Copied!" : stagingPath}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>,
    document.body
  );
}

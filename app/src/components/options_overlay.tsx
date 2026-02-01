// Path: app/src/components/options_overlay.tsx
// Description: Full-screen transparent overlay with options panel for app settings

import type React from "react";
import { useCallback, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import type { GlobalExcludes } from "../shared/global_excludes.js";
import {
  GLOBAL_EXCLUDE_DIR_OPTIONS,
  GLOBAL_EXCLUDE_DIR_SUFFIX_OPTIONS,
  GLOBAL_EXCLUDE_EXTENSION_OPTIONS,
  GLOBAL_EXCLUDE_FILE_OPTIONS,
  GLOBAL_EXCLUDE_SUFFIX_OPTIONS,
  GLOBAL_EXCLUDE_PATTERN_OPTIONS,
  GLOBAL_EXCLUDE_RECOMMENDED_DIRS,
  GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS,
  GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES,
  GLOBAL_EXCLUDE_RECOMMENDED_FILES,
  GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS,
} from "../shared/global_excludes.js";
import "../styles/options_overlay.css";

interface OptionsOverlayProps {
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  stagingPath: string | null;
  globalExcludes: GlobalExcludes;
  setGlobalExcludes: (excludes: GlobalExcludes) => void;
  onClose: () => void;
}

function normalizeExtension(value: string): string {
  const trimmed = value.trim().toLowerCase();
  if (trimmed.length === 0) return "";
  return trimmed.startsWith(".") ? trimmed : `.${trimmed}`;
}

function normalizePattern(value: string): string {
  const trimmed = value.trim().replace(/^\/+|\/+$/g, "").toLowerCase();
  return trimmed;
}

function normalizeName(value: string): string {
  return value.trim().toLowerCase();
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
  const [advancedSectionsOpen, setAdvancedSectionsOpen] = useState({
    directories: true,
    dirSuffixes: true,
    fileNames: true,
    fileSuffixes: true,
    extensions: true,
    patterns: true,
  });

  const toggleAdvanced = useCallback(() => {
    setAdvancedOpen((prev) => !prev);
  }, []);

  const toggleAdvancedSection = useCallback(
    (key: keyof typeof advancedSectionsOpen) => {
      setAdvancedSectionsOpen((prev) => ({ ...prev, [key]: !prev[key] }));
    },
    []
  );

  const normalizedExtensions = useMemo(
    () => globalExcludes.extensions.map(normalizeExtension).filter((value) => value.length > 0),
    [globalExcludes.extensions]
  );
  const normalizedPatterns = useMemo(
    () => globalExcludes.patterns.map(normalizePattern).filter((value) => value.length > 0),
    [globalExcludes.patterns]
  );
  const normalizedDirNames = useMemo(
    () => globalExcludes.dirNames.map(normalizePattern).filter((value) => value.length > 0),
    [globalExcludes.dirNames]
  );
  const normalizedDirSuffixes = useMemo(
    () => globalExcludes.dirSuffixes.map(normalizeExtension).filter((value) => value.length > 0),
    [globalExcludes.dirSuffixes]
  );
  const normalizedFileNames = useMemo(
    () => globalExcludes.fileNames.map(normalizeName).filter((value) => value.length > 0),
    [globalExcludes.fileNames]
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

  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (event.target === event.currentTarget) {
        handleClose();
      }
    },
    [handleClose]
  );

  const recommendedEnabled = useMemo(() => {
    const extensionSet = new Set(normalizedExtensions);
    const patternSet = new Set(normalizedPatterns);
    const dirNameSet = new Set(normalizedDirNames);
    const dirSuffixSet = new Set(normalizedDirSuffixes);
    const fileNameSet = new Set(normalizedFileNames);
    const hasAllExtensions = GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS.every((ext) =>
      extensionSet.has(ext)
    );
    const hasAllFileSuffixes = GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES.every((ext) =>
      extensionSet.has(ext)
    );
    const hasAllPatterns = GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS.every((pattern) =>
      patternSet.has(pattern)
    );
    const hasAllDirNames = GLOBAL_EXCLUDE_RECOMMENDED_DIRS.every((name) =>
      dirNameSet.has(name)
    );
    const hasAllDirSuffixes = GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES.every((suffix) =>
      dirSuffixSet.has(suffix)
    );
    const hasAllFileNames = GLOBAL_EXCLUDE_RECOMMENDED_FILES.every((name) =>
      fileNameSet.has(name)
    );
    return (
      hasAllExtensions &&
      hasAllFileSuffixes &&
      hasAllPatterns &&
      hasAllDirNames &&
      hasAllDirSuffixes &&
      hasAllFileNames
    );
  }, [
    normalizedDirNames,
    normalizedDirSuffixes,
    normalizedExtensions,
    normalizedFileNames,
    normalizedPatterns,
  ]);

  const handleRecommendedToggle = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      const enabled = event.target.checked;
      setGlobalExcludes({
        dirNames: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_DIRS] : [],
        dirSuffixes: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES] : [],
        fileNames: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_FILES] : [],
        extensions: enabled
          ? [...GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS, ...GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES]
          : [],
        patterns: enabled ? [...GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS] : [],
      });
    },
    [setGlobalExcludes]
  );

  const handleExtensionToggle = useCallback(
    (value: string, enabled: boolean) => {
      const normalized = normalizeExtension(value);
      const extensionSet = new Set(normalizedExtensions);
      if (enabled) {
        extensionSet.add(normalized);
      } else {
        extensionSet.delete(normalized);
      }
      setGlobalExcludes({
        dirNames: normalizedDirNames,
        dirSuffixes: normalizedDirSuffixes,
        fileNames: normalizedFileNames,
        extensions: Array.from(extensionSet),
        patterns: normalizedPatterns,
      });
    },
    [
      normalizedDirNames,
      normalizedDirSuffixes,
      normalizedExtensions,
      normalizedFileNames,
      normalizedPatterns,
      setGlobalExcludes,
    ]
  );

  const handlePatternToggle = useCallback(
    (value: string, enabled: boolean) => {
      const normalized = normalizePattern(value);
      const patternSet = new Set(normalizedPatterns);
      if (enabled) {
        patternSet.add(normalized);
      } else {
        patternSet.delete(normalized);
      }
      setGlobalExcludes({
        dirNames: normalizedDirNames,
        dirSuffixes: normalizedDirSuffixes,
        fileNames: normalizedFileNames,
        extensions: normalizedExtensions,
        patterns: Array.from(patternSet),
      });
    },
    [
      normalizedDirNames,
      normalizedDirSuffixes,
      normalizedExtensions,
      normalizedFileNames,
      normalizedPatterns,
      setGlobalExcludes,
    ]
  );

  const handleDirToggle = useCallback(
    (value: string, enabled: boolean) => {
      const normalized = normalizePattern(value);
      const dirSet = new Set(normalizedDirNames);
      if (enabled) {
        dirSet.add(normalized);
      } else {
        dirSet.delete(normalized);
      }
      setGlobalExcludes({
        dirNames: Array.from(dirSet),
        dirSuffixes: normalizedDirSuffixes,
        fileNames: normalizedFileNames,
        extensions: normalizedExtensions,
        patterns: normalizedPatterns,
      });
    },
    [
      normalizedDirNames,
      normalizedDirSuffixes,
      normalizedExtensions,
      normalizedFileNames,
      normalizedPatterns,
      setGlobalExcludes,
    ]
  );

  const handleDirSuffixToggle = useCallback(
    (value: string, enabled: boolean) => {
      const normalized = normalizeExtension(value);
      const suffixSet = new Set(normalizedDirSuffixes);
      if (enabled) {
        suffixSet.add(normalized);
      } else {
        suffixSet.delete(normalized);
      }
      setGlobalExcludes({
        dirNames: normalizedDirNames,
        dirSuffixes: Array.from(suffixSet),
        fileNames: normalizedFileNames,
        extensions: normalizedExtensions,
        patterns: normalizedPatterns,
      });
    },
    [
      normalizedDirNames,
      normalizedDirSuffixes,
      normalizedExtensions,
      normalizedFileNames,
      normalizedPatterns,
      setGlobalExcludes,
    ]
  );

  const handleFileNameToggle = useCallback(
    (value: string, enabled: boolean) => {
      const normalized = normalizeName(value);
      const fileSet = new Set(normalizedFileNames);
      if (enabled) {
        fileSet.add(normalized);
      } else {
        fileSet.delete(normalized);
      }
      setGlobalExcludes({
        dirNames: normalizedDirNames,
        dirSuffixes: normalizedDirSuffixes,
        fileNames: Array.from(fileSet),
        extensions: normalizedExtensions,
        patterns: normalizedPatterns,
      });
    },
    [
      normalizedDirNames,
      normalizedDirSuffixes,
      normalizedExtensions,
      normalizedFileNames,
      normalizedPatterns,
      setGlobalExcludes,
    ]
  );

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
          <div className="options-section-title">EXCLUDES</div>
          <label className="options-checkbox-row">
            <input
              type="checkbox"
              checked={recommendedEnabled}
              onChange={handleRecommendedToggle}
            />
            <span>
              Recommended excludes
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
            <span className="options-section-title">Advanced</span>
            <span className={`options-chevron ${advancedOpen ? "open" : ""}`}>
              ▸
            </span>
          </button>

          {advancedOpen && (
            <div className="options-section-content">
              <div className="options-advanced-group">
                <button
                  type="button"
                  className="options-advanced-toggle"
                  onClick={() => {
                    toggleAdvancedSection("directories");
                  }}
                  aria-expanded={advancedSectionsOpen.directories}
                >
                  <span className="options-advanced-title">Directories</span>
                  <span className={`options-chevron ${advancedSectionsOpen.directories ? "open" : ""}`}>
                    ▸
                  </span>
                </button>
                {advancedSectionsOpen.directories && (
                  <div className="options-advanced-grid">
                    {GLOBAL_EXCLUDE_DIR_OPTIONS.map((option) => {
                      const checked = normalizedDirNames.includes(option.value);
                      return (
                        <label key={option.value} className="options-checkbox-row">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(event) => {
                              handleDirToggle(option.value, event.target.checked);
                            }}
                          />
                          <span>{option.label}</span>
                        </label>
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="options-advanced-group">
                <button
                  type="button"
                  className="options-advanced-toggle"
                  onClick={() => {
                    toggleAdvancedSection("dirSuffixes");
                  }}
                  aria-expanded={advancedSectionsOpen.dirSuffixes}
                >
                  <span className="options-advanced-title">Directory Suffixes</span>
                  <span className={`options-chevron ${advancedSectionsOpen.dirSuffixes ? "open" : ""}`}>
                    ▸
                  </span>
                </button>
                {advancedSectionsOpen.dirSuffixes && (
                  <div className="options-advanced-grid">
                    {GLOBAL_EXCLUDE_DIR_SUFFIX_OPTIONS.map((option) => {
                      const checked = normalizedDirSuffixes.includes(option.value);
                      return (
                        <label key={option.value} className="options-checkbox-row">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(event) => {
                              handleDirSuffixToggle(option.value, event.target.checked);
                            }}
                          />
                          <span>{option.label}</span>
                        </label>
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="options-advanced-group">
                <button
                  type="button"
                  className="options-advanced-toggle"
                  onClick={() => {
                    toggleAdvancedSection("fileNames");
                  }}
                  aria-expanded={advancedSectionsOpen.fileNames}
                >
                  <span className="options-advanced-title">File Names</span>
                  <span className={`options-chevron ${advancedSectionsOpen.fileNames ? "open" : ""}`}>
                    ▸
                  </span>
                </button>
                {advancedSectionsOpen.fileNames && (
                  <div className="options-advanced-grid">
                    {GLOBAL_EXCLUDE_FILE_OPTIONS.map((option) => {
                      const checked = normalizedFileNames.includes(option.value);
                      return (
                        <label key={option.value} className="options-checkbox-row">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(event) => {
                              handleFileNameToggle(option.value, event.target.checked);
                            }}
                          />
                          <span>{option.label}</span>
                        </label>
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="options-advanced-group">
                <button
                  type="button"
                  className="options-advanced-toggle"
                  onClick={() => {
                    toggleAdvancedSection("fileSuffixes");
                  }}
                  aria-expanded={advancedSectionsOpen.fileSuffixes}
                >
                  <span className="options-advanced-title">File Suffixes</span>
                  <span className={`options-chevron ${advancedSectionsOpen.fileSuffixes ? "open" : ""}`}>
                    ▸
                  </span>
                </button>
                {advancedSectionsOpen.fileSuffixes && (
                  <div className="options-advanced-grid">
                    {GLOBAL_EXCLUDE_SUFFIX_OPTIONS.map((option) => {
                      const checked = normalizedExtensions.includes(option.value);
                      return (
                        <label key={option.value} className="options-checkbox-row">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(event) => {
                              handleExtensionToggle(option.value, event.target.checked);
                            }}
                          />
                          <span>{option.label}</span>
                        </label>
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="options-advanced-group">
                <button
                  type="button"
                  className="options-advanced-toggle"
                  onClick={() => {
                    toggleAdvancedSection("extensions");
                  }}
                  aria-expanded={advancedSectionsOpen.extensions}
                >
                  <span className="options-advanced-title">Extensions</span>
                  <span className={`options-chevron ${advancedSectionsOpen.extensions ? "open" : ""}`}>
                    ▸
                  </span>
                </button>
                {advancedSectionsOpen.extensions && (
                  <div className="options-advanced-grid">
                    {GLOBAL_EXCLUDE_EXTENSION_OPTIONS.map((option) => {
                      const checked = normalizedExtensions.includes(option.value);
                      return (
                        <label key={option.value} className="options-checkbox-row">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(event) => {
                              handleExtensionToggle(option.value, event.target.checked);
                            }}
                          />
                          <span>{option.label}</span>
                        </label>
                      );
                    })}
                  </div>
                )}
              </div>

              <div className="options-advanced-group">
                <button
                  type="button"
                  className="options-advanced-toggle"
                  onClick={() => {
                    toggleAdvancedSection("patterns");
                  }}
                  aria-expanded={advancedSectionsOpen.patterns}
                >
                  <span className="options-advanced-title">Path Segments</span>
                  <span className={`options-chevron ${advancedSectionsOpen.patterns ? "open" : ""}`}>
                    ▸
                  </span>
                </button>
                {advancedSectionsOpen.patterns && (
                  <div className="options-advanced-grid">
                    {GLOBAL_EXCLUDE_PATTERN_OPTIONS.map((option) => {
                      const checked = normalizedPatterns.includes(option.value);
                      return (
                        <label key={option.value} className="options-checkbox-row">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(event) => {
                              handlePatternToggle(option.value, event.target.checked);
                            }}
                          />
                          <span>{option.label}</span>
                        </label>
                      );
                    })}
                  </div>
                )}
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

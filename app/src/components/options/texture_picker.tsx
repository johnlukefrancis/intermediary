// Path: app/src/components/options/texture_picker.tsx
// Description: Small texture picker popover for tab theme selection

import type React from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { TextureOption } from "../../lib/theme/texture_catalog.js";

interface TexturePickerProps {
  options: TextureOption[];
  selectedId: string;
  onSelect: (textureId: string) => void;
}

export function TexturePicker({
  options,
  selectedId,
  onSelect,
}: TexturePickerProps): React.JSX.Element {
  const [isOpen, setIsOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const selected = useMemo(
    () => options.find((option) => option.id === selectedId) ?? options[0],
    [options, selectedId]
  );

  const handleToggle = useCallback(() => {
    setIsOpen((prev) => !prev);
  }, []);

  const handleSelect = useCallback(
    (textureId: string) => {
      onSelect(textureId);
      setIsOpen(false);
    },
    [onSelect]
  );

  useEffect(() => {
    if (!isOpen) return;

    const handleClick = (event: MouseEvent): void => {
      const target = event.target as Node;
      if (
        menuRef.current &&
        !menuRef.current.contains(target) &&
        buttonRef.current &&
        !buttonRef.current.contains(target)
      ) {
        setIsOpen(false);
      }
    };

    const handleKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);

    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [isOpen]);

  return (
    <div className="options-texture-picker">
      <button
        ref={buttonRef}
        type="button"
        className="options-texture-button"
        onClick={handleToggle}
        title={selected?.label ?? "Choose texture"}
        aria-haspopup="menu"
        aria-expanded={isOpen}
      >
        <span
          className="options-texture-preview"
          style={{
            backgroundImage: selected ? `url(${selected.url})` : "none",
          }}
          aria-hidden="true"
        />
      </button>

      {isOpen && (
        <div ref={menuRef} className="options-texture-menu" role="menu">
          {options.map((option) => {
            const isSelected = option.id === selectedId;
            return (
              <button
                key={option.id}
                type="button"
                className={`options-texture-option ${
                  isSelected ? "selected" : ""
                }`}
                role="menuitemradio"
                aria-checked={isSelected}
                onClick={() => {
                  handleSelect(option.id);
                }}
                title={option.label}
              >
                <span
                  className="options-texture-thumb"
                  style={{ backgroundImage: `url(${option.url})` }}
                  aria-hidden="true"
                />
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

// Path: app/src/components/options/texture_picker.tsx
// Description: Small texture picker popover for tab theme selection

import type React from "react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { TextureOption } from "../../lib/theme/texture_catalog.js";

interface TexturePickerProps {
  options: TextureOption[];
  selectedId: string;
  onSelect: (textureId: string) => void;
}

interface MenuPosition {
  left: number;
  top: number;
  isReady: boolean;
}

const MENU_GAP_PX = 8;
const VIEWPORT_PADDING_PX = 8;

export function TexturePicker({
  options,
  selectedId,
  onSelect,
}: TexturePickerProps): React.JSX.Element {
  const [isOpen, setIsOpen] = useState(false);
  const [menuPosition, setMenuPosition] = useState<MenuPosition>({
    left: 0,
    top: 0,
    isReady: false,
  });
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const selected = useMemo(
    () => options.find((option) => option.id === selectedId) ?? options[0],
    [options, selectedId]
  );

  const handleToggle = useCallback(() => {
    setIsOpen((prev) => {
      const next = !prev;
      if (next) {
        setMenuPosition((current) => ({
          ...current,
          isReady: false,
        }));
      }
      return next;
    });
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

  useLayoutEffect(() => {
    if (!isOpen) return;

    const updateMenuPosition = (): void => {
      const button = buttonRef.current;
      const menu = menuRef.current;
      if (!button || !menu) return;

      const buttonRect = button.getBoundingClientRect();
      const menuRect = menu.getBoundingClientRect();

      const maxLeft = window.innerWidth - menuRect.width - VIEWPORT_PADDING_PX;
      const maxTop = window.innerHeight - menuRect.height - VIEWPORT_PADDING_PX;
      const desiredLeft = buttonRect.right + MENU_GAP_PX;
      const desiredTop = buttonRect.bottom - menuRect.height;

      const clampedLeft = Math.max(VIEWPORT_PADDING_PX, Math.min(maxLeft, desiredLeft));
      const clampedTop = Math.max(VIEWPORT_PADDING_PX, Math.min(maxTop, desiredTop));

      setMenuPosition((current) => {
        if (
          current.left === clampedLeft &&
          current.top === clampedTop &&
          current.isReady
        ) {
          return current;
        }
        return {
          left: clampedLeft,
          top: clampedTop,
          isReady: true,
        };
      });
    };

    updateMenuPosition();
    window.addEventListener("resize", updateMenuPosition);
    document.addEventListener("scroll", updateMenuPosition, true);

    return () => {
      window.removeEventListener("resize", updateMenuPosition);
      document.removeEventListener("scroll", updateMenuPosition, true);
    };
  }, [isOpen]);

  const textureMenu = isOpen
    ? createPortal(
        <div
          ref={menuRef}
          className="options-texture-menu"
          role="menu"
          style={{
            left: menuPosition.left,
            top: menuPosition.top,
            opacity: menuPosition.isReady ? 1 : 0,
            pointerEvents: menuPosition.isReady ? "auto" : "none",
          }}
        >
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
        </div>,
        document.body
      )
    : null;

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
      {textureMenu}
    </div>
  );
}

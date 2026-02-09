// Path: app/src/components/context_menu.tsx
// Description: Generic reusable right-click context menu with glass aesthetic

import type React from "react";
import { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import { createPortal } from "react-dom";
import "../styles/context_menu.css";

export interface ContextMenuItem {
  label: string;
  onClick: () => void;
}

interface ContextMenuProps {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

export function ContextMenu({
  x,
  y,
  items,
  onClose,
}: ContextMenuProps): React.JSX.Element {
  const menuRef = useRef<HTMLDivElement>(null);

  // Clamp position so the menu stays within the viewport
  useLayoutEffect(() => {
    const el = menuRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const clampedX = Math.min(x, window.innerWidth - rect.width - 4);
    const clampedY = Math.min(y, window.innerHeight - rect.height - 4);
    el.style.left = `${Math.max(0, clampedX)}px`;
    el.style.top = `${Math.max(0, clampedY)}px`;
  }, [x, y]);

  // Dismiss on Escape
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent): void => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => { document.removeEventListener("keydown", handleKeyDown); };
  }, [onClose]);

  const handleBackdropMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) {
        e.preventDefault();
        onClose();
      }
    },
    [onClose]
  );

  const handleItemClick = useCallback(
    (action: () => void) => {
      action();
      onClose();
    },
    [onClose]
  );

  return createPortal(
    <div className="context-menu-backdrop" onMouseDown={handleBackdropMouseDown}>
      <div ref={menuRef} className="context-menu" role="menu" style={{ left: x, top: y }}>
        {items.map((item) => (
          <button
            key={item.label}
            type="button"
            className="context-menu-item"
            role="menuitem"
            onClick={() => { handleItemClick(item.onClick); }}
          >
            {item.label}
          </button>
        ))}
      </div>
    </div>,
    document.body
  );
}

# Intermediary UI Design System

Updated on: 2026-01-31 (Phase 7 QA complete)
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006

---

## Design Principles

**V2: Vintage Instrument Deck** — The UI reads as a physical device, not a floating web app.

1. **Chassis Frame** — The app window has a visible border frame with subtle accent glow, like a hardware faceplate.
2. **Layered Substrate** — Dot grid + grain texture sits BEHIND content (z-index: 0), adding depth without fuzzing text.
3. **Aggressive Dark** — Base palette is ~35% darker than V1, creating maximum contrast and drama.
4. **Hardware Radii** — Corners are crisper (2-4px) to evoke instruments, not soft rounded app UI.
5. **Per-Tab Substrate** — Each tab modulates the substrate's grid dot color and vignette tint.
6. **Token-Driven** — All visual values flow from design tokens. No hardcoded colors in components.

---

## Token Architecture

### Layer Order (Critical)

CSS imports must follow this order in `app/src/main.tsx`:

```
tokens.css        → Abstract primitives (spacing, radii, blur, shadows, typography, motion)
theme_dark.css    → Fills semantic slots with dark theme values
theme_accents.css → Per-tab accent overrides via data-active-tab
effects.css       → Background gradient, grain, glass utilities
motion.css        → Transition presets, reduced-motion support
a11y.css          → Focus rings, disabled states, screen reader utilities
badges.css        → Unified badge primitives
main.css          → Layout reset and base structure
[components]      → Component-specific styles
```

### File Responsibilities

| File | Purpose | LOC |
|------|---------|-----|
| `tokens.css` | Spacing, radii, deck radii, blur, shadows, typography, motion, semantic color slots | ~130 |
| `theme_dark.css` | Dark theme values (V2: aggressive), semantic states, glass surface, deck frame/substrate | ~85 |
| `theme_accents.css` | Per-tab accent + substrate mapping via `[data-active-tab]` | ~50 |
| `effects.css` | Deck chassis frame, substrate (grid + grain at z:0), vignette, glass/glow utilities | ~80 |
| `main.css` | Reset, document sizing, app shell layout | ~90 |

---

## Color Palette

### Backgrounds (V2: Aggressive Dark)

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-bg-base` | `#050508` | App background, deepest layer |
| `--color-bg-surface` | `#0a0a10` | Panels, cards, columns |
| `--color-bg-elevated` | `#0f0f18` | Headers, dropdowns, elevated surfaces |
| `--color-bg-hover` | `#151520` | Hover states |
| `--color-bg-active` | `#1a1a28` | Active/pressed states |

### Text

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-text-primary` | `#e8e8ec` | Body text, primary content |
| `--color-text-secondary` | `#9898a8` | Labels, metadata, secondary info |
| `--color-text-muted` | `#606070` | Disabled text, hints |

### Borders (V2: Adjusted for darker palette)

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-border` | `#1a1a28` | Standard borders |
| `--color-border-subtle` | `#12121a` | Subtle dividers |
| `--color-border-highlight` | `rgba(255,255,255,0.06)` | Glass edge highlights |

### Accents by Tab

| Tab | Primary | Soft | Glow |
|-----|---------|------|------|
| Intermediary | `#c4688a` (dusty rose) | `rgba(196,104,138,0.15)` | `rgba(196,104,138,0.4)` |
| TexturePortal | `#7c3aed` (deep purple) | `rgba(124,58,237,0.15)` | `rgba(124,58,237,0.4)` |
| Triangle Rain | `#16a34a` (muted emerald) | `rgba(22,163,74,0.12)` | `rgba(22,163,74,0.35)` |

**Note**: Each `[data-active-tab]` selector must define both `--accent-*` and `--color-accent-*` variables. This is not duplication — CSS custom properties resolve at definition time, so `--color-accent: var(--accent-primary)` in `:root` captures the root value and won't update when `--accent-primary` changes lower in the tree.

### Semantic States

| State | Primary | Soft | Muted |
|-------|---------|------|-------|
| Success | `#4ade80` | `#1a3d2a` | `#2f7a4b` |
| Error | `#f87171` | `#2a1414` | `#6b1d1d` |
| Info | `#93c5fd` | `#1a2a3d` | `#23324d` |
| Warning | `#fbbf24` | `#2a2414` | `#7a5c1d` |

---

## Spacing Scale

Based on 4px unit:

| Token | Value |
|-------|-------|
| `--space-1` | 4px |
| `--space-2` | 8px |
| `--space-3` | 12px |
| `--space-4` | 16px |
| `--space-5` | 20px |
| `--space-6` | 24px |
| `--space-8` | 32px |

---

## Typography

| Token | Value | Usage |
|-------|-------|-------|
| `--font-sans` | System font stack | Body text |
| `--font-mono` | SF Mono, Fira Code, Consolas | Paths, code, metadata |
| `--text-xs` | 10px | Tiny badges |
| `--text-sm` | 11px | Small labels, status |
| `--text-base` | 13px | Secondary text |
| `--text-md` | 14px | Body text |
| `--text-lg` | 16px | Headers |

---

## V2: Deck Tokens

### Deck Radii (Hardware Feel)

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-deck-none` | 0 | Square corners |
| `--radius-deck-xs` | 2px | Tabs, small controls |
| `--radius-deck-sm` | 3px | Buttons, chips |
| `--radius-deck-md` | 4px | Panels |

### Deck Strokes

| Token | Value | Usage |
|-------|-------|-------|
| `--deck-stroke-hairline` | 1px | Fine details |
| `--deck-stroke-thin` | 2px | Chassis frame |

### Deck Frame

| Token | Value | Usage |
|-------|-------|-------|
| `--deck-frame-inset` | 8px | Substrate margin from window edge |
| `--deck-frame-outer` | `#1a1a24` | Outer frame border color |
| `--deck-frame-inner` | `rgba(255,255,255,0.04)` | Inner highlight stroke |

### Deck Substrate

| Token | Value | Usage |
|-------|-------|-------|
| `--deck-grid-dot` | Per-tab (2-3% opacity) | Dot grid color |
| `--deck-grid-size` | 16px | Dot grid spacing |
| `--deck-grain-opacity` | 0.4 | Grain layer opacity |
| `--deck-vignette-strength` | 0.35 | Edge darkening intensity |
| `--deck-vignette-tint` | Per-tab | Vignette color tint |

---

## Do / Don't

### DO

- Use semantic tokens (`--color-success`) instead of raw hex values
- Apply `.glass-surface` class for frosted panel effects
- Use `var(--color-accent)` for interactive highlights (changes per tab)
- Use `--radius-deck-*` tokens for panel/button corners (V2)
- Test substrate visibility on actual content

### DON'T

- Hardcode hex colors in component CSS
- Use `!important` to override tokens
- Apply backdrop-filter blur to large scrolling areas (performance)
- Mix multiple accent colors within the same component
- Add external image assets for texture effects
- Apply grain as a topmost overlay (causes text fuzzing)

---

## V2: Deck Language Rules (Locked)

These rules define what the V2 deck language is and prevent visual drift.

### What's Forbidden

- `--radius-sm`/`--radius-md`/`--radius-lg` on deck components (use `--radius-deck-*` only)
- Pixel-based letter-spacing (use `em` units: 0.03em tight, 0.05em normal, 0.08em wide)
- Inset focus rings (`outline-offset: -2px`) — always use outset (`2px`) for visibility
- Texture overlays above content (grain/grid must stay at z-index: 0)
- Backdrop-filter on content areas (only on glass surfaces like panels, toasts)

### What's Allowed

- Hardcoded 1-2px micro-spacing for sub-component details (too small for `--space-1`)
- Multi-layer box-shadows for premium button styling (build button pattern)
- Component-specific pixel dimensions (drag handles, toggles, LED dots)
- Inline em-based letter-spacing values (already consistent: 0.03em, 0.05em, 0.08em)

### Must Remain Consistent

- All transitions: `--duration-normal` (150ms) + `--ease-out`
- All focus rings: `outline: 2px solid var(--color-accent); outline-offset: 2px`
- All panels: `--radius-deck-md` corners, multi-layer box-shadow
- All rows: left rail accent with hover glow

---

## Glass Surface Pattern

For frosted glass panels, use the utility class or manual application:

```css
/* Utility class */
.my-panel {
  @extend .glass-surface;
}

/* Manual */
.my-panel {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  box-shadow: var(--shadow-inset-subtle);
}
```

---

## Motion

### Duration Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--duration-instant` | 0ms | Immediate state changes |
| `--duration-fast` | 100ms | Micro-interactions, quick feedback |
| `--duration-normal` | 150ms | Standard transitions (default) |
| `--duration-slow` | 250ms | Deliberate animations |
| `--duration-slower` | 400ms | Page transitions, complex animations |

### Easing Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--ease-out` | `cubic-bezier(0.33, 1, 0.68, 1)` | Enter animations, appearing elements |
| `--ease-in-out` | `cubic-bezier(0.65, 0, 0.35, 1)` | Symmetric animations, pulsing |
| `--ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Bouncy effects (use sparingly) |

### Reduced Motion

All animations and transitions respect `prefers-reduced-motion: reduce`. When enabled:
- Animation duration collapses to near-instant (0.01ms)
- Animation iteration count becomes 1
- Transition duration collapses to near-instant
- Scroll behavior becomes `auto`

This is handled globally in `motion.css` — no per-component opt-out needed.

---

## Accessibility

### Focus Ring Convention

All interactive elements use a consistent `:focus-visible` outline:

```css
.element:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: 2px;
}
```

The base focus state (`:focus`) removes the default outline, and `:focus-visible` adds the styled ring only when keyboard navigation is detected.

### Disabled State Convention

Disabled elements use consistent styling:

```css
.element:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

Some components may also add `filter: saturate(0.5)` for accent-colored buttons.

---

## ASCII Cue Pattern

Panel headers use a monospace `:: ` prefix as a subtle terminal-style decoration:

```css
.panel-header::before {
  content: ':: ';
  color: var(--color-text-muted);
}
```

This is the only ASCII decoration currently in use. Keep ASCII cues minimal and consistent.

---

## Implementation Checklist

Phase 1: Foundation (complete)
- [x] Create `tokens.css`
- [x] Create `theme_dark.css`
- [x] Create `theme_accents.css`
- [x] Create `effects.css`

Phase 2: Wiring (complete)
- [x] Update `main.tsx` import order
- [x] Add `data-active-tab` to `app.tsx`

Phase 3: Migration (complete)
- [x] Refactor `main.css` to layout-only
- [x] Migrate `status_bar.css` (fix --bg-hover bug)
- [x] Migrate `file_row.css`
- [x] Migrate `bundle_column.css`
- [x] Migrate `offline_banner.css`
- [x] Migrate `drag_error_notice.css`
- [x] Migrate `tab_bar.css`
- [x] Migrate `columns.css`

Phase 4: Documentation (complete)
- [x] Create this design doc
- [x] Update `docs/guide.md`

Phase 5: Polish (complete)
- [x] Create `motion.css` with reduced-motion support
- [x] Create `a11y.css` with focus ring utilities
- [x] Add `--color-warning-muted` token
- [x] Replace hardcoded colors with tokens
- [x] Add `:focus-visible` to all interactive elements
- [x] Delete deprecated `offline_banner.css`
- [x] Document accent variable inheritance behavior

---

## V2 Implementation (complete)

Phase 6: Vintage Instrument Deck
- [x] Add deck tokens (`--radius-deck-*`, `--deck-stroke-*`, `--deck-frame-*`, `--deck-grid-*`)
- [x] Aggressive palette darkening (~35% darker)
- [x] Per-tab substrate hooks (`--deck-grid-dot`, `--deck-vignette-tint`)
- [x] Chassis frame on `.app` (box-shadow: outer border + inner highlight + accent glow)
- [x] Substrate layer at z-index: 0 (dot grid + grain, behind content)
- [x] Vignette layer at z-index: 1 (radial gradient edge darkening)
- [x] Update all component radii to use `--radius-deck-*` tokens
- [x] Update this design doc

Phase 7: QA & Polish (complete)
- [x] Audit all UI controls for vintage deck consistency
- [x] Fix pixel-based letter-spacing (`0.5px` → `0.05em`)
- [x] Fix non-deck radii (`--radius-md/sm` → `--radius-deck-sm`)
- [x] Fix inset focus ring (`outline-offset: -2px` → `2px`)
- [x] Add V2 Deck Language Rules section (locked rules)

---

## Future Enhancements

- Light theme variant (`theme_light.css`)
- Additional accent color presets
- Component-specific glass variants (lighter/heavier blur)
- Alternative substrate patterns (line grid, crosshatch)

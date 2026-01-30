# Intermediary UI Design System

Updated on: 2026-01-30
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006

---

## Design Principles

1. **Ink and Glass** — Deep dark backgrounds with frosted glass surfaces create depth without distraction.
2. **Controlled Illumination** — Accent colors provide focal points, not visual noise. Use sparingly.
3. **Quiet Texture** — Subtle grain adds analog warmth without overwhelming content.
4. **Per-Tab Identity** — Each tab has a distinct accent color for quick visual orientation.
5. **Token-Driven** — All visual values flow from design tokens. No hardcoded colors in components.

---

## Token Architecture

### Layer Order (Critical)

CSS imports must follow this order in `app/src/main.tsx`:

```
tokens.css      → Abstract primitives (spacing, radii, blur, shadows, typography, motion)
theme_dark.css  → Fills semantic slots with dark theme values
theme_accents.css → Per-tab accent overrides via data-active-tab
effects.css     → Background gradient, grain, glass utilities
main.css        → Layout reset and base structure
[components]    → Component-specific styles
```

### File Responsibilities

| File | Purpose | LOC |
|------|---------|-----|
| `tokens.css` | Spacing, radii, blur, shadows, typography, motion, semantic color slots | ~110 |
| `theme_dark.css` | Dark theme values, semantic states, glass surface, legacy bridge | ~65 |
| `theme_accents.css` | Per-tab accent mapping via `[data-active-tab]` | ~35 |
| `effects.css` | Background gradient, grain overlay, glass/glow utilities | ~75 |
| `main.css` | Reset, document sizing, app shell layout | ~90 |

---

## Color Palette

### Backgrounds

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-bg-base` | `#0d0d14` | App background, deepest layer |
| `--color-bg-surface` | `#14141f` | Panels, cards, columns |
| `--color-bg-elevated` | `#1a1a28` | Headers, dropdowns, elevated surfaces |
| `--color-bg-hover` | `#222233` | Hover states |
| `--color-bg-active` | `#2a2a3d` | Active/pressed states |

### Text

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-text-primary` | `#e8e8ec` | Body text, primary content |
| `--color-text-secondary` | `#9898a8` | Labels, metadata, secondary info |
| `--color-text-muted` | `#606070` | Disabled text, hints |

### Borders

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-border` | `#2a2a3d` | Standard borders |
| `--color-border-subtle` | `#1f1f2d` | Subtle dividers |
| `--color-border-highlight` | `rgba(255,255,255,0.08)` | Glass edge highlights |

### Accents by Tab

| Tab | Primary | Soft | Glow |
|-----|---------|------|------|
| Intermediary | `#f9a8d4` (light pink) | `rgba(249,168,212,0.15)` | `rgba(249,168,212,0.4)` |
| TexturePortal | `#7c3aed` (deep purple) | `rgba(124,58,237,0.15)` | `rgba(124,58,237,0.4)` |
| Triangle Rain | `#39ff14` (toxic green) | `rgba(57,255,20,0.12)` | `rgba(57,255,20,0.35)` |

### Semantic States

| State | Primary | Soft | Muted |
|-------|---------|------|-------|
| Success | `#4ade80` | `#1a3d2a` | `#2f7a4b` |
| Error | `#f87171` | `#2a1414` | `#6b1d1d` |
| Info | `#93c5fd` | `#1a2a3d` | `#23324d` |
| Warning | `#fbbf24` | `#2a2414` | — |

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

## Do / Don't

### DO

- Use semantic tokens (`--color-success`) instead of raw hex values
- Apply `.glass-surface` class for frosted panel effects
- Use `var(--color-accent)` for interactive highlights (changes per tab)
- Keep grain opacity at 0.025 or lower
- Test backdrop-filter blur on actual scrolling content

### DON'T

- Hardcode hex colors in component CSS
- Use `!important` to override tokens
- Apply backdrop-filter blur to large scrolling areas (performance)
- Mix multiple accent colors within the same component
- Add external image assets for texture effects

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

---

## Future Enhancements

- Light theme variant (`theme_light.css`)
- Additional accent color presets
- Animation tokens for micro-interactions
- Component-specific glass variants (lighter/heavier blur)

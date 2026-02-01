# GitHub Copilot Instructions for mITyFactory

## Project Overview

mITyFactory is a Rust-based application factory with 9 crates and a Tauri 2.0 desktop UI. The UI uses Alpine.js for reactivity and a custom design system.

## Tech Stack

- **Backend**: Rust (9 crates)
- **Desktop**: Tauri 2.0/2.9.5
- **Frontend**: Alpine.js 3.x (CDN), vanilla HTML/CSS
- **Icons**: Inline SVG symbols (Feather-style line icons)
- **Chat**: Agent-based chat system with LLM support (OpenAI/Anthropic)

## Crates

- `mity_core` - Core types and utilities
- `mity_cli` - Command-line interface
- `mity_spec` - Specification management
- `mity_templates` - Project templates
- `mity_iac` - Infrastructure as Code
- `mity_agents` - Agent behaviors
- `mity_runner` - Workflow execution
- `mity_policy` - Policy enforcement
- `mity_chat` - Agent chat system (NEW)
- `mity_ui` - Tauri desktop UI

## Documentation Structure

```
docs/
├── adr/                    # Architecture Decision Records (ADR-XXXX-*.md)
├── architecture/           # Architecture diagrams and docs
│   ├── reference-architecture.md
│   └── workflow-engine.md
└── quickstart.md           # Getting started guide
```

**Important:** ADRs go in `docs/adr/`, NOT in `docs/architecture/adr/`.

Note: Generated projects (from templates) may use a different structure where ADRs are in `docs/architecture/adr/` alongside 4+1 view docs. This guidance applies to the **mITyFactory repo itself**.

## UI Location

All UI files are in `crates/mity_ui/dist/`:
- `index.html` - Main HTML with SVG icon definitions
- `styles.css` - Design system CSS
- `app.js` - Alpine.js application logic

---

## Design System Rules

### Icons - LINE ONLY

**CRITICAL**: Only use line-based (stroke) icons. Never use filled or colored emoji icons.

Icon specifications:
- ViewBox: `0 0 24 24`
- Stroke width: `1.5`
- Stroke: `currentColor`
- Fill: `none`
- Stroke linecap: `round`
- Stroke linejoin: `round`

When adding new icons, add them as SVG symbols in `index.html`:

```html
<symbol id="icon-name" viewBox="0 0 24 24" fill="none" 
        stroke="currentColor" stroke-width="1.5" 
        stroke-linecap="round" stroke-linejoin="round">
    <path d="..."></path>
</symbol>
```

Use icons with:
```html
<svg class="icon-md"><use href="#icon-name"></use></svg>
```

### Available Icons

- `#icon-logo` - Gear/cog (brand)
- `#icon-dashboard` - Grid squares
- `#icon-file` - File with corner fold
- `#icon-workflow` - Refresh arrows
- `#icon-logs` - Horizontal lines
- `#icon-terminal` - Terminal cursor
- `#icon-refresh` - Circular arrow
- `#icon-plus` - Plus sign
- `#icon-apps` - 3D box
- `#icon-specs` - Clipboard
- `#icon-constitution` - Book
- `#icon-folder` - Folder
- `#icon-close` - X mark
- `#icon-check` - Checkmark
- `#icon-alert` - Triangle with !
- `#icon-play` - Play triangle
- `#icon-help` - Circle with ?
- `#icon-info` - Circle with i
- `#icon-settings` - Gear/cog
- `#icon-architecture` - Layered boxes (4+1 architecture)

### Icon Size Classes

```css
.icon-xs   /* 12px - indicators */
.icon-sm   /* 16px - buttons */
.icon-md   /* 20px - navigation */
.icon-lg   /* 32px - features */
.icon-xl   /* 48px - empty states */
.btn-icon  /* 16px - inside buttons */
.nav-icon  /* 18px - sidebar navigation */
```

### Icon Color Classes

```css
.icon-primary   /* var(--color-primary) */
.icon-success   /* var(--color-success) */
.icon-warning   /* var(--color-warning) */
.icon-error     /* var(--color-error) */
.icon-info      /* var(--color-info) */
.icon-muted     /* var(--color-text-muted) */
```

---

## Color System

Always use CSS custom properties, never hardcode colors.

### Primary Palette

```css
--color-bg: #1a1a2e;           /* Main background */
--color-bg-secondary: #16213e; /* Cards, sidebar, header */
--color-bg-tertiary: #0f3460;  /* Hover states, active states */
--color-primary: #e94560;      /* Primary actions, active states */
--color-primary-hover: #ff6b6b;/* Primary button hover */
--color-border: #2a2a4a;       /* Borders, dividers */
```

### Text Colors

```css
--color-text: #eaeaea;         /* Primary text */
--color-text-muted: #a0a0a0;   /* Secondary text, labels */
```

### Semantic Colors

```css
--color-success: #4ade80;      /* Success states */
--color-warning: #fbbf24;      /* Warning states */
--color-error: #ef4444;        /* Error states */
--color-info: #60a5fa;         /* Info states */
```

---

## Typography

```css
--font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
--font-mono: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
```

| Element | Size    | Weight |
|---------|---------|--------|
| H1      | 1.25rem | 600    |
| H2      | 1.5rem  | 600    |
| H3      | 1.1rem  | 600    |
| H4      | 1rem    | 600    |
| Body    | 0.9rem  | 400    |
| Small   | 0.8rem  | 400    |

---

## Component Classes

### Buttons

```html
<button class="btn btn-primary">
    <svg class="btn-icon"><use href="#icon-plus"></use></svg>
    Primary Action
</button>

<button class="btn btn-secondary btn-sm">
    <svg class="btn-icon"><use href="#icon-refresh"></use></svg>
    Refresh
</button>
```

### Cards

```html
<div class="card">Content</div>
<div class="card stat-card">Stats</div>
<div class="card warning-card">Warning</div>
```

### Status Indicators

```html
<span class="status-dot success"></span>
<span class="status-badge success">completed</span>
```

### Forms

```html
<div class="form-group">
    <label>Label</label>
    <input type="text" placeholder="...">
</div>
```

---

## Layout

```
┌─────────────────────────────────────────────────────┐
│ Header (.header)                                    │
├──────────┬──────────────────────────────────────────┤
│ Sidebar  │  Main Content (.content)                 │
│ (.sidebar│                                          │
│  220px)  │  Views: dashboard, specs, workflows,     │
│          │         logs, terminal                   │
└──────────┴──────────────────────────────────────────┘
```

### Spacing

| Size | Value   |
|------|---------|
| xs   | 0.25rem |
| sm   | 0.5rem  |
| md   | 0.75rem |
| base | 1rem    |
| lg   | 1.5rem  |
| xl   | 2rem    |

### Border Radius

- `--radius`: 8px (cards, buttons)
- 4px: small elements
- 9999px: pills, badges

---

## Alpine.js Patterns

The UI uses Alpine.js. Common patterns:

```html
<!-- Conditional rendering -->
<div x-show="currentView === 'dashboard'">

<!-- Click handlers -->
<button @click="loadData()">

<!-- Data binding -->
<span x-text="status.app_count"></span>

<!-- Iteration -->
<template x-for="item in items" :key="item.id">
```

---

## Code Style

### CSS

- Use BEM-like naming: `.component`, `.component-element`
- Always use CSS custom properties for colors
- Group related styles with section comments

### HTML

- Use semantic HTML5 elements
- Use Alpine.js directives for reactivity
- Keep SVG icons in the symbol definitions block

---

## Accessibility (A11Y) Requirements

**CRITICAL**: All UI must meet WCAG 2.1 AA standards.

### Keyboard Navigation

- All interactive elements must be reachable via Tab key
- Logical tab order matching visual layout
- Visible focus indicators (never `outline: none` without alternative)
- Escape key closes modals/dropdowns

### ARIA Guidelines

```html
<!-- Buttons with icon-only need labels -->
<button aria-label="Close dialog" @click="close()">
    <svg class="btn-icon"><use href="#icon-close"></use></svg>
</button>

<!-- Navigation landmarks -->
<nav aria-label="Main navigation">...</nav>
<main role="main">...</main>

<!-- Live regions for dynamic content -->
<div aria-live="polite" aria-atomic="true" x-text="statusMessage"></div>

<!-- Form accessibility -->
<label for="app-name">Application Name</label>
<input id="app-name" type="text" aria-describedby="app-name-hint" required>
<span id="app-name-hint" class="hint">Use lowercase, no spaces</span>
```

### Color Contrast

| Element | Minimum Ratio |
|---------|---------------|
| Normal text | 4.5:1 |
| Large text (18pt+) | 3:1 |
| UI components | 3:1 |
| Focus indicators | 3:1 |

The design system colors are pre-validated:
- `--color-text` on `--color-bg`: ✅ 9.2:1
- `--color-text-muted` on `--color-bg`: ✅ 5.8:1

### Screen Reader Support

- Use semantic HTML (`<button>`, `<nav>`, `<main>`, `<header>`)
- Provide `aria-label` for icon-only buttons
- Use `aria-expanded` for collapsible sections
- Announce loading states with `aria-busy`
- Hide decorative icons with `aria-hidden="true"`

### Motion and Animation

```css
@media (prefers-reduced-motion: reduce) {
    *, *::before, *::after {
        animation-duration: 0.01ms !important;
        transition-duration: 0.01ms !important;
    }
}
```

---

## Testing & Development

Before developing new features, review:
- `.specify/testing-requirements.md` - Testing standards and practices
- `.specify/constitution.md` - Project governance rules
- `.specify/principles.md` - Design principles
- `.specify/features/` - Feature specifications

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p mity_spec

# With coverage
cargo tarpaulin --out Html

# Template smoke tests
cargo run -p mity_cli -- smoke-templates
```

### Pre-Commit Checklist

- [ ] All tests pass (`cargo test --workspace`)
- [ ] Clippy passes (`cargo clippy --workspace`)
- [ ] Format checked (`cargo fmt --check`)
- [ ] New tests for new functionality
- [ ] Documentation updated

---

## DO ✓

- Use CSS custom properties for all colors
- Use line-based SVG icons only
- Follow existing component patterns
- Use semantic color tokens for states
- Add new icons to the symbol definitions block
- Write tests for all new features
- Follow the spec in `.specify/` for feature definitions
- Review constitution before architectural changes
- Place ADRs in `docs/adr/` (NOT `docs/architecture/adr/`)
- Use semantic HTML elements (`<button>`, `<nav>`, `<main>`)
- Provide `aria-label` for icon-only buttons
- Ensure all interactive elements are keyboard accessible
- Test with screen readers before releasing UI changes

## DON'T ✗

- Hardcode color values
- Use filled icons or emojis
- Use inline styles for theming
- Create new color tokens without following the system
- Mix icon styles
- Skip tests for new functionality
- Violate constitution rules
- Remove focus outlines without providing alternatives
- Use color alone to convey information
- Create mouse-only interactions
- Use `div` or `span` for interactive elements (use `button`, `a`)

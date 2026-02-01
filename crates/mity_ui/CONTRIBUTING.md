# Contributing to mITyFactory UI

Welcome to the mITyFactory UI contribution guide. This document provides guidelines for contributing to the user interface, maintaining design consistency, and working with the design system.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Design System](#design-system)
  - [Design Principles](#design-principles)
  - [Color System](#color-system)
  - [Typography](#typography)
  - [Iconography](#iconography)
  - [Components](#components)
  - [Layout](#layout)
- [Code Style](#code-style)
- [Pull Request Process](#pull-request-process)

---

## Getting Started

### Prerequisites

- Rust toolchain (stable)
- Node.js 18+ (for development tools)
- Tauri CLI

### Setup

```bash
# Clone the repository
git clone https://github.com/your-org/mityfactory.git
cd mityfactory

# Build the UI crate
cargo build -p mity_ui

# Run the application with UI
cargo tauri dev
```

### Project Structure

```
crates/mity_ui/
├── dist/
│   ├── index.html      # Main HTML with SVG icon definitions
│   ├── styles.css      # Design system CSS
│   └── app.js          # Alpine.js application logic
├── src/
│   └── lib.rs          # Tauri command handlers
└── CONTRIBUTING.md     # This file
```

---

## Development Workflow

### Making UI Changes

1. **Edit files in `crates/mity_ui/dist/`**
2. **Refresh the Tauri dev window** (automatic with `cargo tauri dev`)
3. **Test across all views** (Dashboard, Specs, Workflows, Logs, Terminal)

### Testing Checklist

- [ ] All 5 views render correctly
- [ ] Icons display properly (line style, correct sizing)
- [ ] Colors match the design system
- [ ] Modal interactions work
- [ ] Responsive behavior is acceptable

---

## Design System

### Design Principles

#### 1. Clarity First
- Use high contrast for important elements
- Clear visual hierarchy guides user attention
- Minimal decoration, maximum function

#### 2. Consistency
- Unified visual language across all views
- Predictable interaction patterns
- Standardized spacing and sizing

#### 3. Professional Aesthetic
- Dark theme optimized for developer focus
- Line-based icons for clean, modern look
- Subtle animations for state feedback

---

### Color System

#### Primary Palette

| Token                  | Value     | Usage                           |
|------------------------|-----------|----------------------------------|
| `--color-bg`           | `#1a1a2e` | Main background                 |
| `--color-bg-secondary` | `#16213e` | Cards, sidebar, header          |
| `--color-bg-tertiary`  | `#0f3460` | Hover states, active states     |
| `--color-primary`      | `#e94560` | Primary actions, active states  |
| `--color-primary-hover`| `#ff6b6b` | Primary button hover            |
| `--color-border`       | `#2a2a4a` | Borders, dividers               |

#### Text Colors

| Token                | Value     | Usage                    |
|----------------------|-----------|--------------------------|
| `--color-text`       | `#eaeaea` | Primary text             |
| `--color-text-muted` | `#a0a0a0` | Secondary text, labels   |

#### Semantic Colors

| Token             | Value     | Usage                         |
|-------------------|-----------|-------------------------------|
| `--color-success` | `#4ade80` | Success states, confirmations |
| `--color-warning` | `#fbbf24` | Warning states, cautions      |
| `--color-error`   | `#ef4444` | Error states, destructive     |
| `--color-info`    | `#60a5fa` | Informational states          |

#### Color Usage

```css
/* ✓ DO: Use CSS custom properties */
.status-success { color: var(--color-success); }

/* ✗ DON'T: Hardcode colors */
.status-success { color: #4ade80; }
```

---

### Typography

#### Font Families

| Token         | Value                                              | Usage              |
|---------------|----------------------------------------------------|--------------------|
| `--font-sans` | `'Inter', -apple-system, BlinkMacSystemFont, ...`  | UI text, labels    |
| `--font-mono` | `'JetBrains Mono', 'Fira Code', 'Consolas', ...`   | Code, terminal     |

#### Type Scale

| Element | Size      | Weight | Usage                   |
|---------|-----------|--------|-------------------------|
| H1      | 1.25rem   | 600    | Brand name (header)     |
| H2      | 1.5rem    | 600    | View titles             |
| H3      | 1.1rem    | 600    | Card titles             |
| H4      | 1rem      | 600    | Section titles          |
| Body    | 0.9rem    | 400    | Default body text       |
| Small   | 0.8rem    | 400    | Labels, captions        |
| Tiny    | 0.75rem   | 400    | Metadata, timestamps    |

---

### Iconography

#### ⚠️ IMPORTANT: Line Icons Only

mITyFactory exclusively uses **line-based (stroke) icons**. Do NOT add filled or colored icons.

#### Icon Specifications

| Property          | Value                    |
|-------------------|--------------------------|
| ViewBox           | `0 0 24 24`             |
| Stroke Width      | `1.5`                   |
| Stroke            | `currentColor`          |
| Fill              | `none`                  |
| Stroke Linecap    | `round`                 |
| Stroke Linejoin   | `round`                 |

#### Icon Sizes

| Class       | Size  | Usage                              |
|-------------|-------|------------------------------------|
| `.icon-xs`  | 12px  | Inline indicators                  |
| `.icon-sm`  | 16px  | Button icons, inline text          |
| `.icon-md`  | 20px  | List items, nav items              |
| `.icon-lg`  | 32px  | Large UI elements                  |
| `.icon-xl`  | 48px  | Empty states, feature highlights   |

#### Available Icons

| Icon ID            | Purpose                    | Symbol Reference         |
|--------------------|----------------------------|--------------------------|
| `#icon-logo`       | Brand/settings             | Gear/cog                 |
| `#icon-dashboard`  | Dashboard view             | Grid squares             |
| `#icon-file`       | Files, documents, specs    | File with corner fold    |
| `#icon-workflow`   | Workflows, processes       | Refresh arrows           |
| `#icon-logs`       | Log entries, lists         | Horizontal lines         |
| `#icon-terminal`   | Terminal, CLI              | Terminal cursor          |
| `#icon-refresh`    | Refresh action             | Circular arrow           |
| `#icon-plus`       | Add/create action          | Plus sign                |
| `#icon-apps`       | Applications, packages     | 3D box                   |
| `#icon-specs`      | Specifications             | Clipboard                |
| `#icon-constitution`| Constitution docs         | Book                     |
| `#icon-folder`     | Directories                | Folder                   |
| `#icon-close`      | Close, dismiss             | X mark                   |
| `#icon-check`      | Success, confirm           | Checkmark                |
| `#icon-alert`      | Warning, caution           | Triangle with !          |
| `#icon-play`       | Run, execute               | Play triangle            |
| `#icon-help`       | Help, info query           | Circle with ?            |
| `#icon-info`       | Information                | Circle with i            |
| `#icon-settings`   | Settings, config           | Gear/cog                 |

#### Icon Usage Examples

```html
<!-- Basic usage -->
<svg class="icon-md"><use href="#icon-dashboard"></use></svg>

<!-- With color class -->
<svg class="icon-md icon-primary"><use href="#icon-check"></use></svg>

<!-- Button icon -->
<button class="btn btn-primary">
    <svg class="btn-icon"><use href="#icon-plus"></use></svg>
    Create App
</button>

<!-- Navigation icon -->
<button class="nav-item">
    <svg class="nav-icon"><use href="#icon-dashboard"></use></svg>
    Dashboard
</button>
```

#### Adding New Icons

1. **Find or create the icon** - Use [Feather Icons](https://feathericons.com/) as reference
2. **Add to symbol definitions** in `index.html`:

```html
<symbol id="icon-newicon" viewBox="0 0 24 24" fill="none" 
        stroke="currentColor" stroke-width="1.5" 
        stroke-linecap="round" stroke-linejoin="round">
    <path d="..."></path>
</symbol>
```

3. **Document the icon** in this file under Available Icons
4. **Use the icon** with the `<svg><use href="#icon-newicon"></use></svg>` pattern

---

### Components

#### Buttons

| Class           | Usage                              |
|-----------------|-----------------------------------|
| `.btn`          | Base button styles                |
| `.btn-primary`  | Primary actions                   |
| `.btn-secondary`| Secondary actions                 |
| `.btn-sm`       | Small buttons                     |

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

#### Cards

| Class           | Usage                              |
|-----------------|-----------------------------------|
| `.card`         | Base card container               |
| `.stat-card`    | Statistics display                |
| `.warning-card` | Warning/alert cards               |

#### Status Indicators

```html
<!-- Status dot -->
<span class="status-dot success"></span>
<span class="status-dot warning"></span>
<span class="status-dot error"></span>

<!-- Status badge -->
<span class="status-badge success">completed</span>
<span class="status-badge warning">running</span>
<span class="status-badge error">failed</span>
```

#### Forms

```html
<div class="form-group">
    <label>Field Label</label>
    <input type="text" placeholder="Enter value...">
</div>

<div class="form-group">
    <label>Select Option</label>
    <select>
        <option value="1">Option 1</option>
    </select>
</div>
```

#### Tables

```html
<table class="table">
    <thead>
        <tr>
            <th>Column 1</th>
            <th>Column 2</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td>Data 1</td>
            <td>Data 2</td>
        </tr>
    </tbody>
</table>
```

#### Modals

```html
<div class="modal-backdrop">
    <div class="modal">
        <div class="modal-header">
            <h3>Modal Title</h3>
            <button class="modal-close">
                <svg class="icon-sm"><use href="#icon-close"></use></svg>
            </button>
        </div>
        <div class="modal-body">
            Modal content here
        </div>
        <div class="modal-footer">
            <button class="btn btn-secondary">Cancel</button>
            <button class="btn btn-primary">Confirm</button>
        </div>
    </div>
</div>
```

#### Toast Notifications

```html
<div class="toast success">
    <svg class="toast-icon"><use href="#icon-check"></use></svg>
    Operation completed successfully
</div>

<div class="toast error">
    <svg class="toast-icon"><use href="#icon-close"></use></svg>
    Something went wrong
</div>
```

---

### Layout

#### Application Structure

```
┌─────────────────────────────────────────────────────┐
│ Header                                              │
├──────────┬──────────────────────────────────────────┤
│          │                                          │
│ Sidebar  │  Main Content                            │
│ (220px)  │                                          │
│          │                                          │
│          │                                          │
│          │                                          │
└──────────┴──────────────────────────────────────────┘
```

#### Spacing Scale

| Size   | Value   | Usage                          |
|--------|---------|--------------------------------|
| xs     | 0.25rem | Tight spacing                  |
| sm     | 0.5rem  | Small gaps                     |
| md     | 0.75rem | Medium spacing                 |
| base   | 1rem    | Default padding                |
| lg     | 1.5rem  | Section padding                |
| xl     | 2rem    | Large sections                 |
| 2xl    | 3rem    | Empty states                   |

#### Border Radius

| Value   | Usage                          |
|---------|--------------------------------|
| 4px     | Small elements                 |
| 8px     | Cards, buttons (`--radius`)    |
| 9999px  | Pills, badges                  |

---

## Code Style

### HTML

- Use semantic HTML5 elements
- Use Alpine.js directives for reactivity (`x-data`, `x-show`, `@click`)
- Keep templates readable with proper indentation

```html
<div class="view" x-show="currentView === 'dashboard'">
    <h2>Dashboard</h2>
    <div class="card">
        <!-- Card content -->
    </div>
</div>
```

### CSS

- Use BEM-like naming: `.component`, `.component-element`, `.component--modifier`
- Always use CSS custom properties for colors
- Group related styles with section comments

```css
/* =============================================================================
   Component Name
   ============================================================================= */

.component {
    color: var(--color-text);
    background: var(--color-bg-secondary);
}

.component-element {
    /* child element styles */
}
```

### JavaScript (Alpine.js)

- Keep `app()` function organized
- Use meaningful method names
- Handle errors gracefully with toast notifications

```javascript
function app() {
    return {
        // State
        loading: false,
        
        // Methods
        async loadData() {
            this.loading = true;
            try {
                // Implementation
            } catch (error) {
                this.showToast('Error loading data', 'error');
            } finally {
                this.loading = false;
            }
        }
    };
}
```

---

## Pull Request Process

### Before Submitting

1. **Test all views** for visual consistency
2. **Verify icons** are line-based (not filled)
3. **Check color usage** uses CSS variables
4. **Run the build** to ensure no errors

### PR Checklist

```markdown
## UI Changes

- [ ] Tested in all views (Dashboard, Specs, Workflows, Logs, Terminal)
- [ ] Icons are line-based (stroke only, no fills)
- [ ] Colors use CSS custom properties
- [ ] Responsive/overflow behavior tested
- [ ] CONTRIBUTING.md updated (if adding new components/icons)

## Screenshots

<!-- Add before/after screenshots for visual changes -->
```

### Review Criteria

- Design consistency with existing UI
- Proper use of design tokens
- Accessibility considerations
- Performance impact

---

## Quick Reference

### Icon Color Classes

```css
.icon-primary   /* var(--color-primary) */
.icon-success   /* var(--color-success) */
.icon-warning   /* var(--color-warning) */
.icon-error     /* var(--color-error) */
.icon-info      /* var(--color-info) */
.icon-muted     /* var(--color-text-muted) */
```

### DO ✓

- Use CSS custom properties for colors
- Apply line icons consistently
- Maintain visual hierarchy with proper spacing
- Use semantic color tokens for states
- Follow the established type scale
- Document new components/icons

### DON'T ✗

- Hardcode color values
- Mix icon styles (no filled icons)
- Use inline styles for theming
- Create new color tokens without documentation
- Override established component patterns

---

## Resources

- **Feather Icons**: [feathericons.com](https://feathericons.com/)
- **Alpine.js Docs**: [alpinejs.dev](https://alpinejs.dev/)
- **Tauri Docs**: [tauri.app](https://tauri.app/)

---

## Questions?

For design questions or clarifications, open a discussion in the repository or reach out to the maintainers.

Thank you for contributing to mITyFactory! 🏭

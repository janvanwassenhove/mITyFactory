# mITyFactory Testing Requirements

## Purpose

This document defines the testing requirements for developing new features in mITyFactory. All contributors must follow these guidelines to ensure code quality and reliability.

---

## Test Categories

### 1. Unit Tests

**Required for**: All public functions, structs, and modules.

**Coverage targets**:
- Core business logic: 90%+
- Public APIs: 100%
- Utility functions: 80%+

**Location**: `src/` alongside code (Rust convention) or `tests/` for integration.

**Naming convention**: `test_<function_name>_<scenario>`

```rust
#[test]
fn test_parse_spec_valid_yaml() { ... }

#[test]
fn test_parse_spec_invalid_returns_error() { ... }
```

---

### 2. Integration Tests

**Required for**: Crate-level interactions and external dependencies.

**Location**: `crates/<crate_name>/tests/`

**What to test**:
- Cross-crate communication
- File system operations
- Container interactions (with mocks)
- Spec → Workflow → Output flows

---

### 3. Documentation Tests

**Required for**: All public API examples in doc comments.

**Rust convention**: Examples in `///` doc comments are automatically tested.

```rust
/// Parses a specification file.
/// 
/// # Example
/// 
/// ```
/// use mity_spec::parse_spec;
/// let spec = parse_spec("path/to/spec.yaml").unwrap();
/// ```
pub fn parse_spec(path: &str) -> Result<Spec, Error> { ... }
```

---

### 4. Template Tests

**Required for**: All project templates.

**Run via**: `mity smoke-templates`

**What to verify**:
- Template renders without errors
- Generated project builds successfully
- Generated tests pass
- Container builds correctly

---

### 5. Accessibility Tests (A11Y)

**Required for**: All UI components in mity_ui and generated frontend templates.

**Standards**: WCAG 2.1 AA compliance.

**What to test**:
- Keyboard navigation (all interactive elements reachable via Tab)
- Focus management (visible focus indicators, logical focus order)
- Screen reader compatibility (ARIA labels, roles, live regions)
- Color contrast (minimum 4.5:1 for text, 3:1 for large text/UI)
- Alternative text for images and icons
- Form accessibility (labels, error messages, required indicators)
- Motion/animation (respect `prefers-reduced-motion`)

**Tools**:
- axe-core for automated accessibility testing
- Manual testing with screen readers (NVDA, VoiceOver)
- Browser DevTools accessibility audit

**Example test pattern**:
```typescript
import { axe, toHaveNoViolations } from 'jest-axe';

expect.extend(toHaveNoViolations);

test('component is accessible', async () => {
  const { container } = render(<MyComponent />);
  const results = await axe(container);
  expect(results).toHaveNoViolations();
});
```

---

### 6. UI JavaScript Tests

**Required for**: All critical UI functionality in mity_ui.

**Location**: `crates/mity_ui/tests/` (E2E tests using Tauri's test framework or Playwright)

**What to test**:
- Navigation flows (Dashboard → Workspace → Project creation)
- Button click handlers (New Project, Run, etc.)
- State management (Alpine.js reactive data)
- Form submissions and validations
- Error handling and toast notifications

**Critical User Journeys to Test**:
1. **New Project Flow**: Dashboard → Click "New Project" → Workspace view appears → Enter description → Project starts
2. **Session Resume Flow**: Dashboard → Click existing project → Session restores
3. **Settings Flow**: Dashboard → Settings → Save changes → Settings persist

**Test Pattern for UI Functions**:
```javascript
// tests/ui/newProject.spec.js
describe('New Project', () => {
  test('clicking New Project from dashboard header switches to workspace view', async () => {
    // Setup: Be on dashboard with existing projects
    app.currentView = 'dashboard';
    app.chat.sessions = [{ id: '123', app_name: 'test' }];
    
    // Action
    await app.startNewProject();
    
    // Assert
    expect(app.currentView).toBe('workspace');
    expect(app.chat.activeSession).toBeNull();
    expect(app.factory.runtime).toBeNull();
  });

  test('clicking New Project clears all session state', async () => {
    // Setup: Active session with data
    app.chat.activeSession = { id: '123' };
    app.factory.runtime = { runState: 'running' };
    app.chat.messages = [{ content: 'test' }];
    
    // Action
    await app.startNewProject();
    
    // Assert
    expect(app.chat.messages).toEqual([]);
    expect(app.chat.loading).toBe(false);
  });
});
```

**Regression Prevention**:
When fixing any UI bug:
1. First write a failing test that reproduces the bug
2. Fix the bug
3. Verify the test passes
4. Keep the test to prevent regression

---

## Test Quality Standards

### ✅ DO

1. **Test one thing per test** - Each test should verify a single behavior
2. **Use descriptive names** - Test names should explain what's being tested
3. **Test edge cases** - Empty inputs, nulls, boundary conditions
4. **Test error conditions** - Verify error messages and types
5. **Keep tests fast** - Unit tests should run in milliseconds
6. **Make tests deterministic** - No reliance on external state or timing

### ❌ DON'T

1. Test implementation details (test behavior, not internals)
2. Write tests that depend on execution order
3. Use real network calls without mocking
4. Hardcode absolute paths
5. Skip error path testing
6. Write tests without assertions

---

## Running Tests

### All Tests
```bash
cargo test --workspace
```

### Specific Crate
```bash
cargo test -p mity_spec
```

### With Coverage
```bash
cargo tarpaulin --out Html
```

### Template Smoke Tests
```bash
cargo run -p mity_cli -- smoke-templates
```

---

## Pre-Commit Checklist

Before committing any feature:

- [ ] All existing tests pass (`cargo test --workspace`)
- [ ] New tests written for new functionality
- [ ] Coverage is maintained or improved
- [ ] Clippy passes (`cargo clippy --workspace`)
- [ ] Format checked (`cargo fmt --check`)
- [ ] Doc tests pass (`cargo test --doc`)

---

## Test Infrastructure

### Fixtures

Test fixtures live in `crates/<crate>/tests/fixtures/`

### Mocks

Use the `mockall` crate for mocking traits:

```rust
#[automock]
trait ContainerRunner {
    fn run(&self, image: &str) -> Result<(), Error>;
}
```

### Test Utilities

Common test utilities are in `mity_core::test_utils` (when needed).

---

## Continuous Integration

All tests run automatically on:
- Pull requests
- Pushes to main
- Nightly builds

CI must pass before merging.

---

*Last Updated: 2026-01-21*

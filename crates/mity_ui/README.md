# mITyFactory Desktop UI

A lightweight Tauri-based desktop UI for mITyFactory.

## Architecture

This UI is a **thin shell** that wraps the `mity` CLI. It contains **no business logic** - all operations are delegated to the CLI via Tauri commands.

```
┌─────────────────────────────────────────┐
│              Tauri UI                    │
│  ┌─────────────────────────────────────┐│
│  │         Web Frontend                 ││
│  │   (HTML/CSS/JS + Alpine.js)          ││
│  └─────────────────┬───────────────────┘│
│                    │ invoke()            │
│  ┌─────────────────▼───────────────────┐│
│  │       Rust Commands Layer            ││
│  │   (src/commands.rs)                  ││
│  └─────────────────┬───────────────────┘│
└────────────────────┼────────────────────┘
                     │ shell out
                     ▼
              ┌──────────────┐
              │   mity CLI   │
              └──────────────┘
```

## Features

- **Dashboard**: Factory status, quick actions
- **Specifications**: Browse and view spec files
- **Workflows**: Monitor running workflows
- **Logs**: View CLI logs
- **Terminal**: Execute CLI commands directly

## Prerequisites

1. [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)
2. Rust toolchain
3. `mity` CLI in PATH (or set `MITY_CLI_PATH` env var)

## Development

```bash
# Install Tauri CLI
cargo install tauri-cli

# Run in development mode
cd crates/mity_ui
cargo tauri dev

# Build for production
cargo tauri build
```

## Building

The UI is **optional** and not included in the main workspace by default. To build:

1. Uncomment `"crates/mity_ui"` in the workspace `Cargo.toml`
2. Run `cargo tauri build` from the `crates/mity_ui` directory

## Design Principles

1. **No Business Logic**: All operations go through the CLI
2. **Thin Wrapper**: UI only handles presentation
3. **CLI-First**: Desktop UI is optional; CLI is always primary
4. **Offline-Ready**: Works without network (CLI handles everything)

## File Structure

```
crates/mity_ui/
├── Cargo.toml          # Rust dependencies
├── build.rs            # Tauri build script
├── tauri.conf.json     # Tauri configuration
├── src/
│   ├── main.rs         # Tauri app entry point
│   └── commands.rs     # CLI wrapper commands
├── dist/               # Frontend assets
│   ├── index.html      # Main HTML
│   ├── styles.css      # Styling
│   └── app.js          # Alpine.js application
└── icons/              # App icons (placeholder)
```

## Tauri Commands

| Command | Description |
|---------|-------------|
| `get_factory_status` | Check if factory is initialized |
| `list_specs` | List all specification files |
| `get_spec_content` | Read a spec file's content |
| `list_workflows` | List active workflows |
| `get_workflow_status` | Get workflow details |
| `run_cli_command` | Execute arbitrary CLI command |
| `get_logs` | Retrieve recent logs |
| `init_factory` | Initialize the factory |
| `create_app` | Create a new application |
| `validate_app` | Validate an application |

## Customization

### Environment Variables

- `MITY_CLI_PATH`: Path to the mity CLI binary (default: `mity`)

### Theming

Edit `dist/styles.css` CSS variables:

```css
:root {
    --color-primary: #e94560;
    --color-bg: #1a1a2e;
    /* ... */
}
```

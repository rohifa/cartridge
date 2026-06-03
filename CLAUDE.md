# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Cartridge** is a local-first Mac desktop application for gamers with libraries scattered across Steam, GOG, and Epic Games. It consolidates all owned games into one unified, browsable shelf with cover art, metadata, and user-layered ratings, tags, notes, and categories—all stored locally in a single file without cloud sync or subscriptions.

## Technology Stack

- **Frontend**: SvelteKit 2 + Svelte 5 (Runes API)
- **Desktop Runtime**: Tauri v2
- **Backend**: Rust (in `src-tauri/`)
- **Build Tool**: Vite 6
- **Package Manager**: pnpm
- **Styling**: Custom CSS design system (see `design system/` directory)

## Architecture

### Tauri + SvelteKit Integration

This is a **Tauri v2** desktop app using **SvelteKit as a static site generator (SSG)**. Key architectural points:

- **No SSR**: Tauri doesn't have a Node.js server, so `ssr = false` and `prerender = true` in `src/routes/+layout.ts`
- **Static Adapter**: Uses `@sveltejs/adapter-static` to prerender the entire app
- **Dev Server**: Vite runs on port 1420 during development (configured in `vite.config.js`)
- **Frontend-Backend Communication**: Svelte components invoke Rust commands via `@tauri-apps/api/core`

### Directory Structure

```
cartridge/
├── src/                    # SvelteKit frontend
│   ├── routes/            # SvelteKit file-based routing
│   │   ├── +layout.ts     # Root layout (prerender + ssr config)
│   │   └── +page.svelte   # Home page
│   └── app.html           # HTML shell
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Tauri app setup, command registration
│   │   └── main.rs        # Entry point
│   ├── Cargo.toml         # Rust dependencies
│   ├── tauri.conf.json    # Tauri configuration
│   └── capabilities/      # Tauri security capabilities
├── static/                # Static assets (icons, images)
├── design system/         # Complete design system documentation
│   ├── README.md          # Design philosophy, visual foundations
│   ├── colors_and_type.css # All design tokens
│   └── ui_kits/           # UI component examples
└── package.json           # Node dependencies
```

### Rust-Frontend Communication

- **Commands**: Rust functions decorated with `#[tauri::command]` in `src-tauri/src/lib.rs`
- **Invocation**: Frontend calls commands via `invoke("command_name", { args })`
- **Registration**: Commands must be registered in `tauri::generate_handler![...]` in `lib.rs`

Example:
```rust
// Rust side (lib.rs)
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Register in Builder
.invoke_handler(tauri::generate_handler![greet])
```

```typescript
// Svelte side
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("greet", { name: "World" });
```

## Development Commands

```bash
# Install dependencies
pnpm install

# Run dev server (starts both Vite and Tauri)
pnpm tauri dev

# Build frontend only
pnpm build

# Build the complete Tauri app bundle
pnpm tauri build

# Type-check Svelte components
pnpm check

# Type-check in watch mode
pnpm check:watch

# Preview production build (frontend only)
pnpm preview
```

### Cargo Commands (Rust Backend)

```bash
# From src-tauri/ directory
cd src-tauri

# Check Rust code
cargo check

# Run Rust tests
cargo test

# Format Rust code
cargo fmt

# Lint Rust code
cargo clippy
```

## Design System

Cartridge has a comprehensive design system located in `design system/`. Key principles:

### Visual Identity
- **Palette**: Near-monochrome graphite (cool gunmetal) with a single warm gold accent
- **Philosophy**: "Dense but calm" — cover art provides color; chrome stays neutral
- **Inspiration**: Tactile cartridge nostalgia meets utility (early iTunes, file managers)

### Design Tokens
All tokens are defined in `design system/colors_and_type.css`:
- **Typography**: Geist (UI) + Geist Mono (data/metadata)
- **Spacing**: 4px base grid (`--space-1` through `--space-12`)
- **Colors**: Dark mode primary, warm paper light mode
- **Accent**: Gold (`--gold-500`) for selection, focus, ratings
- **Platform Colors**: Small dots only (Steam blue, GOG violet, Epic white)

### Content Tone
- **Voice**: Calm, precise librarian — factual, never marketing-speak
- **Casing**: Sentence case everywhere (never Title Case)
- **Numbers**: Always precise in monospace (`142.6 h`, `1,284 games`)
- **Emoji**: Never used in UI
- **Honesty**: Plain language for gaps ("No cover art", "Playtime not tracked")

### Key Files
- `design system/README.md` — Full design philosophy and guidelines
- `design system/colors_and_type.css` — All CSS custom properties
- `design system/ui_kits/cartridge/` — Interactive UI examples

**When implementing UI**: Reference the design system tokens and maintain the "dense but calm" aesthetic. Cover art is the primary visual element; chrome should be restrained and utilitarian.

## Svelte 5 Specifics

This project uses **Svelte 5** with the new **Runes API**:
- Reactive state: `let count = $state(0)`
- Computed values: `let doubled = $derived(count * 2)`
- Effects: `$effect(() => { ... })`
- Props: `let { propName } = $props()`

See existing components in `src/routes/` for examples.

## Tauri Configuration

Key settings in `src-tauri/tauri.conf.json`:
- **Product Name**: `cartridge`
- **Identifier**: `com.cartridge.app`
- **Dev URL**: `http://localhost:1420`
- **Frontend Dist**: `../build` (SvelteKit output)
- **Window**: 800x600 default size

## File-Based Routing

SvelteKit uses file-based routing in `src/routes/`:
- `+page.svelte` → page component
- `+layout.svelte` → layout wrapper
- `+layout.ts` / `+page.ts` → load functions, config
- `+server.ts` → API endpoints (not applicable in Tauri static mode)

Since this is a desktop app with SSG, all routes are prerendered at build time.

## Adding Tauri Plugins

The project uses `tauri-plugin-opener` for opening external links. To add more plugins:

1. Add to `src-tauri/Cargo.toml`:
   ```toml
   tauri-plugin-name = "2"
   ```

2. Register in `src-tauri/src/lib.rs`:
   ```rust
   .plugin(tauri_plugin_name::init())
   ```

3. Add frontend types (if needed):
   ```bash
   pnpm add @tauri-apps/plugin-name
   ```

## Important Notes

- **Local-First**: All data stored on user's machine, no cloud sync
- **Security**: Tauri's CSP is currently `null` in config — consider tightening for production
- **Platform**: Primary target is macOS (may need updates for Windows/Linux)
- **Build Output**: Rust target directory (`src-tauri/target/`) is large; excluded from git

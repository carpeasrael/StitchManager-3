# StitchManager

A cross-platform desktop application for managing embroidery files. Built with Tauri v2, Rust, and TypeScript.

StitchManager helps embroidery enthusiasts and professionals organize, browse, and enrich their stitch file collections with format-aware parsing, AI-powered metadata analysis, and batch operations.

## Features

### Format Parsing
- Native binary parsing for **PES**, **DST**, **JEF**, and **VP3** embroidery formats
- Extracts stitch count, dimensions, thread colors, and color palettes (Brother, Janome)
- Embedded thumbnail extraction (PES) with synthetic stitch-render fallback
- Thread color database with manufacturer-specific color mapping

### File Management
- Folder-based library with directory scanning and automatic file detection
- Real-time file watcher — new files added to watched folders appear instantly
- Multi-select with batch rename, batch organize (directory structure from patterns), and USB export
- Configurable naming patterns with variables: `{name}`, `{theme}`, `{format}`, `{index}`
- Mass import with progress tracking and runtime display
- "Go to location" — reveal files in your OS file manager
- File attachments and unique IDs per embroidery file
- PDF report generation per file

### Search
- Global cross-folder search via "Alle Ordner" sidebar entry
- Advanced search across all file parameters (name, theme, tags, format, etc.)
- Debounced search bar with instant results

### USB Export
- USB drive detection with automatic device monitoring
- Single-file and batch export to removable media

### AI-Powered Analysis
- Integrates with **Ollama** (local) or **OpenAI** for vision-based metadata extraction
- Analyzes embroidery thumbnails to suggest name, theme, description, tags, and colors
- Preview and edit prompt before sending
- Review results per-field, accept or reject individually
- Batch analysis across multiple files with progress tracking

### Metadata & Tags
- Edit name, theme, description, and license per file
- Tag system with dedicated TagInput component and autocomplete
- User-defined custom fields (text, number, date) configurable in settings
- Dirty-state tracking with save indicator

### Migration
- Built-in 2stitch Organizer migration tool for importing existing collections

### UI & Design
- **Aurora** light and dark theme with full design-token system (WCAG AA compliant)
- Custom background support
- Three-panel layout: sidebar (folders), center (file list), right (metadata detail)
- Burger menu for streamlined navigation
- Draggable splitter handles with persisted panel widths
- Resizable settings dialog
- Virtual scrolling for large collections (1000+ files)
- Zoomable image preview dialog
- Keyboard shortcuts (Ctrl+S save, Ctrl+F search, Ctrl+K AI, arrow navigation, and more)
- Toast notifications for user feedback

## Tech Stack

| Layer | Technology |
|---|---|
| Runtime | [Tauri v2](https://v2.tauri.app/) |
| Backend | Rust |
| Frontend | TypeScript + [Vite](https://vitejs.dev/) (vanilla, no framework) |
| Database | SQLite via `rusqlite` + `tauri-plugin-sql` |
| AI | Ollama / OpenAI (vision API) via `reqwest` |
| File Watching | `notify` crate |
| PDF | `printpdf` + `qrcode` crates |

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install) (1.77.2+)
- Tauri v2 system dependencies — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

## Getting Started

```bash
# Clone the repository
git clone https://github.com/carpeasrael/StitchManager-3.git
cd StitchManager-3

# Install frontend dependencies
npm install

# Run in development mode (starts Vite + Tauri)
npm run tauri dev

# Build for production
npm run tauri build
```

### Other Useful Commands

```bash
# Start Vite dev server only (frontend)
npm run dev

# TypeScript check + Vite production build (frontend only)
npm run build

# Check Rust backend compiles
cd src-tauri && cargo check

# Run Rust tests
cd src-tauri && cargo test
```

## Project Structure

```
src/                        # Frontend (TypeScript)
  components/               # UI components (Sidebar, FileList, MetadataPanel, ...)
  services/                 # Tauri invoke wrappers (FileService, AiService, ...)
  state/                    # AppState (pub/sub) + EventBus
  styles/                   # Aurora theme tokens, layout, component styles
  types/                    # TypeScript type definitions
  utils/                    # Formatting helpers
  shortcuts.ts              # Keyboard shortcut definitions
  main.ts                   # App entry point
  styles.css                # Main stylesheet

src-tauri/                  # Backend (Rust)
  src/
    commands/               # Tauri command handlers (files, folders, ai, batch, settings, scanner, migration, thread_colors)
    db/                     # SQLite database, migrations, models, queries
    parsers/                # Embroidery format parsers (PES, DST, JEF, VP3)
    services/               # Thumbnail generation, AI client, file watcher, PDF reports, USB monitor, thread DB
    error.rs                # Error types
    lib.rs                  # Tauri app setup and plugin registration
    main.rs                 # Binary entry point
  capabilities/             # Tauri permission grants
  tauri.conf.json           # Tauri window, build, and bundle config

docs/
  analysis/                 # Sprint analysis documents
  reviews/                  # Code review logs
```

## Supported Formats

| Format | Extension | Parse | Thumbnail | Colors |
|---|---|---|---|---|
| Brother/Babylock PES | `.pes` | Full | Embedded + synthetic | Brother palette (64) |
| Tajima DST | `.dst` | Full | Synthetic | From header |
| Janome JEF | `.jef` | Full | Synthetic | Janome palette (78) |
| Viking/Pfaff VP3 | `.vp3` | Best-effort | Synthetic | Embedded RGB |

## License

GPL-3.0 — see [LICENSE](LICENSE) for details.

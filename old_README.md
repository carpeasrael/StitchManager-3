# StitchManager

A cross-platform desktop application for managing embroidery files. Built with Tauri v2, Rust, and TypeScript.

StitchManager helps embroidery enthusiasts and professionals organize, browse, and enrich their stitch file collections with format-aware parsing, AI-powered metadata analysis, and batch operations.

## Features

### Format Parsing
- Native binary parsing for **PES**, **DST**, **JEF**, and **VP3** embroidery formats
- Extracts stitch count, dimensions, thread colors, and color palettes (Brother/Peltier, Janome)
- Embedded thumbnail extraction (PES) with synthetic stitch-render fallback

### File Management
- Folder-based library with directory scanning and automatic file detection
- Real-time file watcher — new files added to watched folders appear instantly
- Multi-select with batch rename, batch organize (directory structure from patterns), and USB export
- Configurable naming patterns with variables: `{name}`, `{theme}`, `{format}`, `{index}`

### AI-Powered Analysis
- Integrates with **Ollama** (local) or **OpenAI** for vision-based metadata extraction
- Analyzes embroidery thumbnails to suggest name, theme, description, tags, and colors
- Preview prompt before sending, review results per-field, accept or reject individually
- Batch analysis across multiple files with progress tracking

### Metadata & Tags
- Edit name, theme, description, and license per file
- Tag system with autocomplete from existing tags
- User-defined custom fields (text, number, date) configurable in settings
- Dirty-state tracking with save indicator

### UI & Design
- **Aurora** light and dark theme with full design-token system (WCAG AA compliant)
- Three-panel layout: sidebar (folders), center (file list), right (metadata detail)
- Draggable splitter handles with persisted panel widths
- Virtual scrolling for large collections (1000+ files)
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

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install) (1.77.2+)
- Tauri v2 system dependencies — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

## Getting Started

```bash
# Clone the repository
git clone https://github.com/carpeasrael/StitchManager-2.git
cd StitchManager-2

# Install frontend dependencies
npm install

# Run in development mode (starts Vite + Tauri)
npm run tauri dev

# Build for production
npm run tauri build
```

## Project Structure

```
src/                        # Frontend (TypeScript)
  components/               # UI components (Sidebar, FileList, MetadataPanel, ...)
  dialogs/                  # Modal dialogs (Settings, AI Preview/Result, Batch)
  services/                 # Tauri invoke wrappers (FileService, AiService, ...)
  state/                    # AppState (pub/sub) + EventBus
  styles/                   # Aurora theme tokens, layout, component styles
  utils/                    # Keyboard shortcuts, splitter, virtual scroll
  main.ts                   # App entry point

src-tauri/                  # Backend (Rust)
  src/
    commands/               # Tauri command handlers (files, folders, ai, batch, ...)
    db/                     # SQLite database, migrations, models
    parsers/                # Embroidery format parsers (PES, DST, JEF, VP3)
    services/               # Thumbnail generation, AI client, file watcher
    lib.rs                  # Tauri app setup and plugin registration
```

## Supported Formats

| Format | Extension | Parse | Thumbnail | Colors |
|---|---|---|---|---|
| Brother/Babylock PES | `.pes` | Full | Embedded + synthetic | Brother palette (64) |
| Tajima DST | `.dst` | Full | Synthetic | From header |
| Janome JEF | `.jef` | Full | Synthetic | Janome palette (78) |
| Viking/Pfaff VP3 | `.vp3` | Best-effort | Synthetic | Embedded RGB |

## License

ISC

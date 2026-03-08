# StichMan v2 — Technisches Proposal

> Rebuild von Python/Qt nach Rust/Tauri v2
> Autor: CarpeAsrael · Datum: 2026-03-06
> Release-Zyklus: `26.03-a1`

---

## Inhaltsverzeichnis

1. [Zusammenfassung](#1-zusammenfassung)
2. [Architektur-Uebersicht](#2-architektur-uebersicht)
3. [Datenbankschema](#3-datenbankschema)
4. [Backend-Design (Rust / Tauri)](#4-backend-design-rust--tauri)
5. [Frontend-Architektur (TypeScript / Vanilla)](#5-frontend-architektur-typescript--vanilla)
6. [Design-System-Integration](#6-design-system-integration)
7. [Implementierungsphasen](#7-implementierungsphasen)
8. [Migrationshinweise](#8-migrationshinweise)

---

## 1. Zusammenfassung

### 1.1 Ausgangslage

StichMan v1.0.0 ist eine macOS-Applikation zur Verwaltung von Stickdateien, gebaut mit:

- **Python 3.9** + **PySide6/Qt6** fuer die GUI
- **PyInstaller** fuer das App-Bundle (~110 MB)
- **NumPy** + **Pillow** fuer Bildverarbeitung
- **SQLite** (Runtime-Datenbank, ausserhalb des Bundles)
- Zielplattform: macOS ARM64 (Apple Silicon)

Das bestehende Bundle enthaelt 71 MB PySide6-Frameworks, 12 MB PIL, 6.7 MB NumPy und 6.5 MB Python-Extensions — der Grossteil der Bundle-Groesse stammt aus den Python-Abhaengigkeiten, nicht aus der Anwendungslogik.

### 1.2 Warum ein Rebuild?

| Aspekt | v1 (Python/Qt) | v2 (Rust/Tauri) |
|---|---|---|
| Bundle-Groesse | ~110 MB | ~15–20 MB |
| Startzeit | 3–5 s (Python-Interpreter + Qt-Initialisierung) | < 1 s |
| Plattformen | macOS ARM64 | macOS, Windows, Linux |
| UI-Technologie | PySide6 (C++ Widgets) | HTML/CSS/TypeScript (Web-Renderer) |
| Speichersicherheit | Python GC + C-Extensions | Rust Ownership-System |
| AI-Integration | Direkte HTTP-Aufrufe | Strukturierte Tauri-Commands |
| Theme-System | Qt-Stylesheets | CSS Custom Properties (Aurora Design Tokens) |

### 1.3 Ziele des Rebuilds

1. **Bundle-Groesse < 20 MB** durch Wegfall der Python-Runtime und Qt-Frameworks
2. **Cross-Platform** — ein Codebase fuer macOS, Windows und Linux
3. **Native Performance** — Rust-Backend fuer Dateisystem-Scans, PES/DST-Parsing, Thumbnail-Generierung
4. **Modernes UI** — CSS-basiertes Aurora-Theme-System gemaess dem Design-Proposal (`design/design-proposal.md`)
5. **Feature-Paritaet** mit v1, plus verbesserte AI-Workflows und Batch-Operationen

---

## 2. Architektur-Uebersicht

### 2.1 Systemarchitektur

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri Window                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Frontend (WebView)                   │  │
│  │  TypeScript · Vanilla Components · Aurora CSS     │  │
│  │                                                   │  │
│  │  AppState ←→ EventBus ←→ Service Layer            │  │
│  └──────────────────┬────────────────────────────────┘  │
│                     │ invoke() / listen()                │
│                     │ (Tauri IPC)                        │
│  ┌──────────────────┴────────────────────────────────┐  │
│  │              Backend (Rust)                        │  │
│  │  Tauri Commands · File Parsers · AI Client        │  │
│  │  Directory Scanner · Thumbnail Generator          │  │
│  │                                                   │  │
│  │  rusqlite ←→ SQLite (stitch_manager.db)           │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 2.2 IPC-Modell

Tauri v2 verwendet zwei IPC-Mechanismen:

- **`invoke(command, args)`** — Frontend ruft Backend-Funktionen synchron auf (Request/Response)
- **`listen(event)` / `emit(event)`** — Bidirektionaler Event-Bus fuer asynchrone Benachrichtigungen (z.B. Scan-Fortschritt, AI-Ergebnisse)

### 2.3 Datenbank-Zugriffsstrategie

Die aktuelle Scaffold-Konfiguration verwendet `tauri-plugin-sql` fuer den Frontend-seitigen DB-Zugriff. Fuer v2 wird eine **duale Strategie** verfolgt:

| Zugriffspfad | Verwendung |
|---|---|
| **Backend (rusqlite)** | Schreiboperationen, Migrationen, komplexe Queries, Batch-Operationen, Scanner-Ergebnisse |
| **Frontend (tauri-plugin-sql)** | Leichtgewichtige Lese-Queries fuer UI-Updates, Settings-Zugriff |

Das Backend uebernimmt die Schema-Migration und stellt sicher, dass alle Schreibzugriffe validiert und transaktional sind.

### 2.4 Verzeichnisstruktur (Ziel)

```
src/
├── main.ts                    # App-Einstiegspunkt, Routing
├── components/
│   ├── Component.ts           # Basisklasse fuer alle Komponenten
│   ├── Sidebar.ts             # Ordner-Navigation
│   ├── FileList.ts            # Dateiliste mit Mini-Cards
│   ├── MetadataPanel.ts       # Rechtes Panel: Vorschau + Formular
│   ├── Toolbar.ts             # Aktions-Toolbar
│   ├── SearchBar.ts           # Such-Eingabe mit Filter-Chips
│   ├── StatusBar.ts           # Untere Statusleiste
│   └── FilterChips.ts         # Format- und Tag-Filter
├── dialogs/
│   ├── SettingsDialog.ts      # Einstellungen (Tabs)
│   ├── AiPreviewDialog.ts     # AI-Prompt-Vorschau
│   ├── AiResultDialog.ts      # AI-Ergebnis-Review
│   └── BatchDialog.ts         # Batch-Fortschritt
├── services/
│   ├── FileService.ts         # invoke()-Wrapper: Dateien
│   ├── FolderService.ts       # invoke()-Wrapper: Ordner
│   ├── ScannerService.ts      # invoke()-Wrapper: Scanner
│   ├── AiService.ts           # invoke()-Wrapper: KI-Analyse
│   ├── SettingsService.ts     # invoke()-Wrapper: Einstellungen
│   └── BatchService.ts        # invoke()-Wrapper: Batch-Ops
├── state/
│   ├── AppState.ts            # Zentraler reaktiver State-Store
│   └── EventBus.ts            # Frontend-interner Event-Bus
├── types/
│   └── index.ts               # Shared TypeScript-Interfaces
└── styles/
    ├── aurora.css             # Design-Tokens als CSS Custom Properties
    ├── components.css         # Komponenten-Styles
    ├── layout.css             # Grid-Layout, Panel-Sizing
    └── dialogs.css            # Dialog-Styles

src-tauri/
└── src/
    ├── lib.rs                 # App-Einstiegspunkt, Plugin-Registrierung
    ├── db/
    │   ├── mod.rs             # DB-Modul, Connection-Pool
    │   ├── migrations.rs      # Schema-Migrationen
    │   └── models.rs          # Rust-Structs fuer DB-Tabellen
    ├── commands/
    │   ├── mod.rs             # Command-Registrierung
    │   ├── files.rs           # Datei-Commands
    │   ├── folders.rs         # Ordner-Commands
    │   ├── scanner.rs         # Directory-Scanner-Commands
    │   ├── ai.rs              # AI-Analyse-Commands
    │   ├── batch.rs           # Batch-Operations
    │   └── settings.rs        # Einstellungs-Commands
    ├── parsers/
    │   ├── mod.rs             # Parser-Trait + Registry
    │   ├── pes.rs             # PES-Format-Parser
    │   ├── dst.rs             # DST-Format-Parser
    │   ├── jef.rs             # JEF-Format-Parser
    │   └── vp3.rs             # VP3-Format-Parser
    ├── services/
    │   ├── thumbnail.rs       # Thumbnail-Generierung
    │   ├── ai_client.rs       # Ollama/OpenAI HTTP-Client
    │   └── file_watcher.rs    # Dateisystem-Ueberwachung
    └── error.rs               # Zentrales Error-Handling
```

---

## 3. Datenbankschema

Die SQLite-Datenbank `stitch_manager.db` wird beim ersten App-Start erstellt. Alle Migrationen werden ueber die `schema_version`-Tabelle versioniert.

### 3.1 schema_version

Verwaltet die aktuelle Schema-Version fuer inkrementelle Migrationen.

```sql
CREATE TABLE schema_version (
    version     INTEGER NOT NULL,
    applied_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    description TEXT
);

INSERT INTO schema_version (version, description)
VALUES (1, 'Initial schema');
```

### 3.2 folders

Verwaltete Ordner-Eintraege (nicht das Dateisystem selbst, sondern die Zuordnung zu logischen Ordnern in der App).

```sql
CREATE TABLE folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    path        TEXT    NOT NULL UNIQUE,
    parent_id   INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_folders_parent ON folders(parent_id);
CREATE INDEX idx_folders_path   ON folders(path);
```

### 3.3 embroidery_files

Haupttabelle fuer alle Stickdateien mit Metadaten.

```sql
CREATE TABLE embroidery_files (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id       INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    filename        TEXT    NOT NULL,
    filepath        TEXT    NOT NULL UNIQUE,
    name            TEXT,                     -- Anzeigename (editierbar)
    theme           TEXT,                     -- Thema / Motiv
    description     TEXT,                     -- Beschreibung (ggf. KI-generiert)
    license         TEXT,                     -- Lizenz-Info
    width_mm        REAL,                     -- Breite in mm
    height_mm       REAL,                     -- Hoehe in mm
    stitch_count    INTEGER,                  -- Anzahl Stiche
    color_count     INTEGER,                  -- Anzahl Farben
    file_size_bytes INTEGER,                  -- Dateigroesse
    thumbnail_path  TEXT,                     -- Pfad zum generierten Thumbnail
    ai_analyzed     INTEGER NOT NULL DEFAULT 0,  -- 0 = nein, 1 = ja
    ai_confirmed    INTEGER NOT NULL DEFAULT 0,  -- 0 = unbestaetigt, 1 = bestaetigt
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_files_folder   ON embroidery_files(folder_id);
CREATE INDEX idx_files_name     ON embroidery_files(name);
CREATE INDEX idx_files_filepath ON embroidery_files(filepath);
CREATE INDEX idx_files_ai       ON embroidery_files(ai_analyzed);
```

### 3.4 file_formats

Zuordnung von Dateiformaten zu einer Stickdatei (eine Datei kann in mehreren Formaten vorliegen, z.B. PES + DST).

```sql
CREATE TABLE file_formats (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id         INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
    format          TEXT    NOT NULL,            -- 'PES', 'DST', 'JEF', 'VP3', etc.
    format_version  TEXT,                        -- z.B. 'v5' fuer PES
    filepath        TEXT    NOT NULL,            -- Pfad zur spezifischen Format-Datei
    file_size_bytes INTEGER,
    parsed          INTEGER NOT NULL DEFAULT 0   -- 0 = nicht geparst, 1 = geparst
);

CREATE INDEX idx_formats_file   ON file_formats(file_id);
CREATE INDEX idx_formats_format ON file_formats(format);
```

### 3.5 file_thread_colors

Fadenfarbeninformationen pro Datei.

```sql
CREATE TABLE file_thread_colors (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id     INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    color_hex   TEXT    NOT NULL,                -- '#8B4513'
    color_name  TEXT,                            -- 'Sattlebraun'
    brand       TEXT,                            -- 'Madeira', 'Guetermann', etc.
    brand_code  TEXT,                            -- Hersteller-Farbnummer
    is_ai       INTEGER NOT NULL DEFAULT 0       -- 0 = aus Datei, 1 = KI-generiert
);

CREATE INDEX idx_colors_file ON file_thread_colors(file_id);
```

### 3.6 tags / file_tags

Tag-System mit n:m-Zuordnung.

```sql
CREATE TABLE tags (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE file_tags (
    file_id INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (file_id, tag_id)
);

CREATE INDEX idx_file_tags_file ON file_tags(file_id);
CREATE INDEX idx_file_tags_tag  ON file_tags(tag_id);
```

### 3.7 ai_analysis_results

Speichert KI-Analyse-Ergebnisse fuer Audit und Vergleich.

```sql
CREATE TABLE ai_analysis_results (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id         INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
    provider        TEXT    NOT NULL,          -- 'ollama', 'openai'
    model           TEXT    NOT NULL,          -- 'llama3.2-vision', 'gpt-4o', etc.
    prompt_hash     TEXT,                      -- SHA-256 des gesendeten Prompts
    raw_response    TEXT,                      -- Rohe KI-Antwort (JSON)
    parsed_name     TEXT,                      -- Extrahierter Name
    parsed_theme    TEXT,                      -- Extrahiertes Thema
    parsed_desc     TEXT,                      -- Extrahierte Beschreibung
    parsed_tags     TEXT,                      -- JSON-Array: ["Weihnachten","Rentier"]
    parsed_colors   TEXT,                      -- JSON-Array der erkannten Farben
    accepted        INTEGER NOT NULL DEFAULT 0,
    analyzed_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_ai_file ON ai_analysis_results(file_id);
```

### 3.8 settings

Key-Value-Store fuer Applikationseinstellungen.

```sql
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Default-Einstellungen
INSERT INTO settings (key, value) VALUES
    ('library_root',    '~/Stickdateien'),
    ('metadata_root',   '~/Stickdateien/.stichman'),
    ('theme_mode',      'hell'),
    ('ai_provider',     'ollama'),
    ('ai_url',          'http://localhost:11434'),
    ('ai_model',        'llama3.2-vision'),
    ('ai_temperature',  '0.3'),
    ('ai_timeout_ms',   '30000'),
    ('rename_pattern',  '{name}_{theme}'),
    ('organize_pattern', '{theme}/{name}');
```

### 3.9 custom_field_definitions / custom_field_values

Benutzerdefinierte Metadatenfelder.

```sql
CREATE TABLE custom_field_definitions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE,
    field_type  TEXT    NOT NULL DEFAULT 'text',   -- 'text', 'number', 'date', 'select'
    options     TEXT,                               -- JSON-Array fuer 'select'-Typ
    required    INTEGER NOT NULL DEFAULT 0,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE custom_field_values (
    file_id     INTEGER NOT NULL REFERENCES embroidery_files(id) ON DELETE CASCADE,
    field_id    INTEGER NOT NULL REFERENCES custom_field_definitions(id) ON DELETE CASCADE,
    value       TEXT,
    PRIMARY KEY (file_id, field_id)
);

CREATE INDEX idx_custom_values_file ON custom_field_values(file_id);
```

---

## 4. Backend-Design (Rust / Tauri)

### 4.1 Zusaetzliche Cargo-Abhaengigkeiten

Die folgenden Crates werden zum bestehenden `Cargo.toml` hinzugefuegt:

```toml
[dependencies]
# Bestehend:
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
tauri = { version = "2.10" }
tauri-plugin-log = "2"
tauri-plugin-sql = { version = "2", features = ["sqlite"] }

# Neu:
rusqlite = { version = "0.31", features = ["bundled"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
notify = "6"                    # Dateisystem-Watcher
image = "0.25"                  # Thumbnail-Generierung
walkdir = "2"                   # Rekursiver Directory-Scan
chrono = { version = "0.4", features = ["serde"] }
sha2 = "0.10"                  # Prompt-Hashing
base64 = "0.22"                # Bild-Encoding fuer AI
uuid = { version = "1", features = ["v4"] }
thiserror = "1"                # Error-Handling
byteorder = "1"                # Binary-Parsing (Stickformate)
```

### 4.2 Tauri-Commands

Alle Commands werden als `#[tauri::command]`-Funktionen implementiert und in `lib.rs` via `.invoke_handler(tauri::generate_handler![...])` registriert.

#### 4.2.1 Modul: `commands/files.rs`

```rust
#[tauri::command]
async fn get_files(folder_id: Option<i64>, search: Option<String>,
                   format_filter: Option<String>) -> Result<Vec<EmbroideryFile>, AppError>

#[tauri::command]
async fn get_file(file_id: i64) -> Result<EmbroideryFile, AppError>

#[tauri::command]
async fn update_file(file_id: i64, updates: FileUpdate) -> Result<EmbroideryFile, AppError>

#[tauri::command]
async fn delete_file(file_id: i64) -> Result<(), AppError>

#[tauri::command]
async fn get_file_formats(file_id: i64) -> Result<Vec<FileFormat>, AppError>

#[tauri::command]
async fn get_file_colors(file_id: i64) -> Result<Vec<ThreadColor>, AppError>

#[tauri::command]
async fn get_file_tags(file_id: i64) -> Result<Vec<Tag>, AppError>

#[tauri::command]
async fn set_file_tags(file_id: i64, tag_names: Vec<String>) -> Result<Vec<Tag>, AppError>

#[tauri::command]
async fn get_thumbnail(file_id: i64) -> Result<String, AppError>  // Base64-encoded
```

#### 4.2.2 Modul: `commands/folders.rs`

```rust
#[tauri::command]
async fn get_folders() -> Result<Vec<Folder>, AppError>

#[tauri::command]
async fn create_folder(name: String, path: String,
                       parent_id: Option<i64>) -> Result<Folder, AppError>

#[tauri::command]
async fn update_folder(folder_id: i64, name: Option<String>) -> Result<Folder, AppError>

#[tauri::command]
async fn delete_folder(folder_id: i64) -> Result<(), AppError>

#[tauri::command]
async fn get_folder_file_count(folder_id: i64) -> Result<i64, AppError>
```

#### 4.2.3 Modul: `commands/scanner.rs`

```rust
#[tauri::command]
async fn scan_directory(path: String,
                        app_handle: tauri::AppHandle) -> Result<ScanResult, AppError>
// Emittiert Events: "scan:progress", "scan:file-found", "scan:complete"

#[tauri::command]
async fn import_files(file_paths: Vec<String>,
                      folder_id: i64) -> Result<Vec<EmbroideryFile>, AppError>

#[tauri::command]
async fn parse_embroidery_file(filepath: String) -> Result<ParsedFileInfo, AppError>
```

#### 4.2.4 Modul: `commands/ai.rs`

```rust
#[tauri::command]
async fn ai_analyze_file(file_id: i64,
                         app_handle: tauri::AppHandle) -> Result<AiAnalysisResult, AppError>
// Emittiert Events: "ai:start", "ai:complete", "ai:error"

#[tauri::command]
async fn ai_analyze_batch(file_ids: Vec<i64>,
                          app_handle: tauri::AppHandle) -> Result<Vec<AiAnalysisResult>, AppError>

#[tauri::command]
async fn ai_accept_result(result_id: i64) -> Result<EmbroideryFile, AppError>

#[tauri::command]
async fn ai_reject_result(result_id: i64) -> Result<(), AppError>

#[tauri::command]
async fn ai_build_prompt(file_id: i64) -> Result<AiPromptPreview, AppError>

#[tauri::command]
async fn ai_test_connection() -> Result<AiConnectionStatus, AppError>
```

#### 4.2.5 Modul: `commands/batch.rs`

```rust
#[tauri::command]
async fn batch_rename(file_ids: Vec<i64>, pattern: String,
                      app_handle: tauri::AppHandle) -> Result<BatchResult, AppError>
// Emittiert Events: "batch:progress", "batch:complete"

#[tauri::command]
async fn batch_organize(file_ids: Vec<i64>, pattern: String,
                        app_handle: tauri::AppHandle) -> Result<BatchResult, AppError>

#[tauri::command]
async fn batch_export_usb(file_ids: Vec<i64>,
                          target_path: String) -> Result<BatchResult, AppError>
```

#### 4.2.6 Modul: `commands/settings.rs`

```rust
#[tauri::command]
async fn get_setting(key: String) -> Result<String, AppError>

#[tauri::command]
async fn set_setting(key: String, value: String) -> Result<(), AppError>

#[tauri::command]
async fn get_all_settings() -> Result<HashMap<String, String>, AppError>

#[tauri::command]
async fn get_custom_fields() -> Result<Vec<CustomFieldDef>, AppError>

#[tauri::command]
async fn create_custom_field(name: String, field_type: String,
                             options: Option<Vec<String>>) -> Result<CustomFieldDef, AppError>

#[tauri::command]
async fn delete_custom_field(field_id: i64) -> Result<(), AppError>
```

### 4.3 Embroidery-Parser-Trait

Die verschiedenen Stickformate werden ueber ein einheitliches Trait abstrahiert:

```rust
pub struct ParsedFileInfo {
    pub format: String,
    pub format_version: Option<String>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub stitch_count: Option<u32>,
    pub color_count: Option<u16>,
    pub colors: Vec<ParsedColor>,
}

pub struct ParsedColor {
    pub hex: String,
    pub name: Option<String>,
    pub brand: Option<String>,
    pub brand_code: Option<String>,
}

pub trait EmbroideryParser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, ParseError>;
    fn extract_thumbnail(&self, data: &[u8]) -> Result<Option<Vec<u8>>, ParseError>;
}
```

Implementierungen:

| Format | Crate/Methode | Besonderheiten |
|---|---|---|
| **PES** | Eigenimplementierung (`byteorder`) | Eingebettete Thumbnails, Farbpalette mit Marken-Codes |
| **DST** | Eigenimplementierung | Kein eingebettetes Thumbnail, Farben aus Header extrahierbar |
| **JEF** | Eigenimplementierung | Janome-spezifische Farbpalette |
| **VP3** | Eigenimplementierung | Pfaff/Viking-Format, komplexe Farbsektionen |

Fuer Formate ohne eingebettetes Thumbnail wird der `image`-Crate verwendet, um aus den Stich-Koordinaten ein synthetisches Vorschaubild zu rendern.

### 4.4 AI-Client

Der AI-Client unterstuetzt zwei Provider:

```rust
pub enum AiProvider {
    Ollama { base_url: String },
    OpenAi { base_url: String, api_key: String },
}

pub struct AiClient {
    provider: AiProvider,
    model: String,
    temperature: f32,
    timeout: Duration,
    http: reqwest::Client,
}

impl AiClient {
    pub async fn analyze(&self, image_base64: &str,
                         prompt: &str) -> Result<AiResponse, AiError>;
    pub async fn test_connection(&self) -> Result<bool, AiError>;
}
```

Der Prompt wird mit Datei-Metadaten (Abmessungen, Stichzahl, Farben) angereichert und zusammen mit dem Thumbnail-Bild an das Vision-Modell gesendet.

### 4.5 Thumbnail-Generierung

```rust
pub struct ThumbnailGenerator {
    cache_dir: PathBuf,   // {metadata_root}/thumbnails/
    target_size: (u32, u32),  // 192 x 192 px
}

impl ThumbnailGenerator {
    pub fn generate(&self, file_id: i64, data: &[u8],
                    parser: &dyn EmbroideryParser) -> Result<PathBuf, ThumbnailError>;
    pub fn get_cached(&self, file_id: i64) -> Option<PathBuf>;
    pub fn invalidate(&self, file_id: i64) -> Result<(), ThumbnailError>;
}
```

Strategie:
1. **PES**: Eingebettetes Thumbnail extrahieren, auf Zielgroesse skalieren
2. **Andere Formate**: Stich-Koordinaten parsen, in ein `image::RgbaImage` rendern, speichern
3. Thumbnails werden im `{metadata_root}/thumbnails/`-Verzeichnis gecacht

### 4.6 Error-Handling

Zentraler Fehlertyp mit `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Datenbankfehler: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Dateifehler: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parserfehler: {format} — {message}")]
    Parse { format: String, message: String },

    #[error("KI-Fehler: {0}")]
    Ai(String),

    #[error("Nicht gefunden: {0}")]
    NotFound(String),

    #[error("Validierungsfehler: {0}")]
    Validation(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}
```

---

## 5. Frontend-Architektur (TypeScript / Vanilla)

### 5.1 Komponenten-System

Da kein Framework verwendet wird, definieren wir eine leichtgewichtige Basisklasse:

```typescript
abstract class Component {
    protected el: HTMLElement;
    private subscriptions: Array<() => void> = [];

    constructor(container: HTMLElement) {
        this.el = container;
    }

    abstract render(): void;

    protected subscribe(event: string, handler: (data: any) => void): void {
        const unsub = EventBus.on(event, handler);
        this.subscriptions.push(unsub);
    }

    destroy(): void {
        this.subscriptions.forEach(unsub => unsub());
        this.el.innerHTML = '';
    }
}
```

Jede Komponente erhaelt ihren Container-DOM-Knoten, rendert sich selbst und kann Events abonnieren.

### 5.2 Reaktiver State-Store

```typescript
interface State {
    folders: Folder[];
    selectedFolderId: number | null;
    files: EmbroideryFile[];
    selectedFileId: number | null;
    searchQuery: string;
    formatFilter: string | null;
    settings: Record<string, string>;
    theme: 'hell' | 'dunkel';
}

class AppState {
    private state: State;
    private listeners: Map<string, Set<(value: any) => void>>;

    get<K extends keyof State>(key: K): State[K];
    set<K extends keyof State>(key: K, value: State[K]): void;  // Benachrichtigt Listener
    on<K extends keyof State>(key: K, listener: (value: State[K]) => void): () => void;
}
```

Wenn ein State-Wert via `set()` geaendert wird, werden alle registrierten Listener benachrichtigt. Komponenten binden sich an State-Keys und re-rendern bei Aenderungen.

### 5.3 EventBus

```typescript
class EventBus {
    private static handlers: Map<string, Set<Function>>;

    static emit(event: string, data?: any): void;
    static on(event: string, handler: Function): () => void;  // Returns unsubscribe fn
}
```

Der EventBus verbindet Komponenten untereinander und leitet Tauri-Backend-Events weiter:

```typescript
// Tauri-Events an den Frontend-Bus weiterleiten
import { listen } from '@tauri-apps/api/event';

listen('scan:progress', (e) => EventBus.emit('scan:progress', e.payload));
listen('ai:complete',   (e) => EventBus.emit('ai:complete',   e.payload));
listen('batch:progress',(e) => EventBus.emit('batch:progress', e.payload));
```

### 5.4 Service-Layer

Jeder Service kapselt `invoke()`-Aufrufe ans Backend:

```typescript
import { invoke } from '@tauri-apps/api/core';

export class FileService {
    static async getFiles(folderId?: number, search?: string,
                          formatFilter?: string): Promise<EmbroideryFile[]> {
        return invoke('get_files', {
            folderId: folderId ?? null,
            search: search ?? null,
            formatFilter: formatFilter ?? null
        });
    }

    static async getFile(fileId: number): Promise<EmbroideryFile> {
        return invoke('get_file', { fileId });
    }

    static async updateFile(fileId: number, updates: FileUpdate): Promise<EmbroideryFile> {
        return invoke('update_file', { fileId, updates });
    }

    static async getThumbnail(fileId: number): Promise<string> {
        return invoke('get_thumbnail', { fileId });
    }
    // ... weitere Methoden
}
```

### 5.5 Hauptkomponenten

| Komponente | Verantwortung | State-Bindings |
|---|---|---|
| **Sidebar** | Ordner-Liste, Auswahl, Datei-Zaehler, "+ Neuer Ordner" | `folders`, `selectedFolderId` |
| **SearchBar** | Suchfeld, Debounced Input | `searchQuery` |
| **FilterChips** | Format-Filter (Alle, PES, DST, JEF, VP3) | `formatFilter` |
| **FileList** | Mini-Cards mit Thumbnail, Name, Format-Chips, AI-Badge | `files`, `selectedFileId` |
| **MetadataPanel** | Vorschau-Bild, Formularfelder, Farb-Swatches, Aktions-Buttons | `selectedFileId` |
| **Toolbar** | Datei/Ordner hinzufuegen, Speichern, Einstellungen, KI Analyse | — |
| **StatusBar** | Datei-Zaehler nach Format, aktueller Ordner | `files`, `selectedFolderId` |

### 5.6 Dialoge

Dialoge werden als modale Overlays implementiert:

```typescript
abstract class Dialog extends Component {
    protected overlay: HTMLElement;
    protected dialog: HTMLElement;

    show(): void {
        this.overlay = document.createElement('div');
        this.overlay.className = 'dialog-overlay';
        this.dialog = document.createElement('div');
        this.dialog.className = 'dialog';
        this.overlay.appendChild(this.dialog);
        document.body.appendChild(this.overlay);
        this.render();
    }

    close(): void {
        this.overlay.remove();
        this.destroy();
    }
}
```

| Dialog | Groesse | Inhalt |
|---|---|---|
| **SettingsDialog** | 720 x 560 | Tabs: Allgemein, Erscheinungsbild, KI, Dateiverwaltung, Benutzerdefiniert |
| **AiPreviewDialog** | 800 x 600 | Split-View: Prompt-Text links, Datei-Vorschau rechts |
| **AiResultDialog** | 640 x 500 | Checkbox-Felder pro KI-Ergebnis, Farb-Vergleich |
| **BatchDialog** | 480 x 400 | Fortschrittsbalken, Log-View, Step-Indikator |

### 5.7 Layout (CSS Grid)

```css
.app-layout {
    display: grid;
    grid-template-rows: 28px 48px 1fr 22px;
    grid-template-columns: var(--sidebar-width, 240px) var(--center-width, 480px) 1fr;
    grid-template-areas:
        "menu    menu    menu"
        "toolbar toolbar toolbar"
        "sidebar center  right"
        "status  status  status";
    height: 100vh;
    overflow: hidden;
}
```

Die Panel-Breiten werden ueber CSS Custom Properties gesteuert und koennen per JavaScript (Splitter-Drag) angepasst werden.

---

## 6. Design-System-Integration

### 6.1 Aurora CSS Tokens

Das Design-Token-System aus `design/design-proposal.md` wird als CSS Custom Properties implementiert:

```css
:root,
[data-theme="hell"] {
    --color-bg:               #f5f5f7;
    --color-surface:          #ffffff;
    --color-elevated:         #ffffff;
    --color-text:             #111111;
    --color-text-secondary:   #44474f;
    --color-muted:            #7b7c80;
    --color-muted-light:      #b4b7bd;
    --color-accent:           #0a84ff;
    --color-accent-strong:    #086dd6;
    --color-accent-10:        #e8f2ff;
    --color-accent-20:        #cee6ff;
    --color-border:           #d1d5db;
    --color-border-light:     #e5e7eb;
    --color-status-green:     #51cf66;
    --color-status-green-bg:  #dcfce7;
    --color-status-green-text:#2f9e44;
    --color-status-red:       #ff6b6b;

    --font-family: "Helvetica Neue", "Segoe UI", Helvetica, Arial, sans-serif;
    --font-size-display:  20px;
    --font-size-heading:  15px;
    --font-size-body:     13px;
    --font-size-label:    13px;
    --font-size-section:  10px;
    --font-size-caption:  11px;

    --spacing-1:  4px;
    --spacing-2:  8px;
    --spacing-3: 12px;
    --spacing-4: 16px;
    --spacing-5: 20px;
    --spacing-6: 24px;
    --spacing-8: 32px;
    --spacing-12: 48px;

    --radius-input:   6px;
    --radius-card:    8px;
    --radius-dialog: 12px;
    --radius-button:  8px;
    --radius-pill:  999px;
    --radius-swatch:  4px;

    --shadow-xs: 0 1px 3px rgba(0,0,0,0.06);
    --shadow-sm: 0 2px 6px rgba(0,0,0,0.10);
    --shadow-md: 0 4px 16px rgba(0,0,0,0.12);
}

[data-theme="dunkel"] {
    --color-bg:               #0f0f10;
    --color-surface:          #1f1f23;
    --color-elevated:         #242428;
    --color-text:             #f5f5f7;
    --color-text-secondary:   #a0a3ab;
    --color-muted:            #5c5e63;
    --color-accent:           #2d7ff9;
    --color-accent-strong:    #4a94ff;
    --color-border:           #2e2e35;
    --color-border-light:     #27272e;
}
```

Das aktive Theme wird per `data-theme`-Attribut auf `<html>` gesetzt. Beim App-Start wird der gespeicherte `theme_mode`-Wert aus der `settings`-Tabelle geladen.

### 6.2 Komponenten-Styles (Beispiel: File Mini-Card)

```css
.file-card {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    padding: var(--spacing-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border-light);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-xs);
    height: 72px;
    cursor: pointer;
    transition: box-shadow 0.15s, border-color 0.15s;
}

.file-card:hover {
    box-shadow: var(--shadow-sm);
}

.file-card.selected {
    background: var(--color-accent-10);
    border: 1.5px solid var(--color-accent);
}
```

### 6.3 Tauri-Fenster-Konfiguration (Ziel)

```json
{
    "app": {
        "windows": [{
            "title": "StichMan",
            "width": 1440,
            "height": 900,
            "minWidth": 960,
            "minHeight": 640,
            "resizable": true,
            "decorations": true,
            "fullscreen": false
        }]
    }
}
```

---

## 7. Implementierungsphasen

### Phase 1: Fundament (Wochen 1–2)

**Ziel:** Lauffaehige App mit Datenbankschema und Grundstruktur.

Aufgaben:
- [ ] SQLite-Schema implementieren (alle 10 Tabellen, Migrationslogik)
- [ ] Rust-Modulstruktur aufsetzen (`commands/`, `db/`, `parsers/`, `services/`)
- [ ] `AppError`-Typ und Serialisierung
- [ ] Frontend-Grundgeruest: `Component`-Basisklasse, `AppState`, `EventBus`
- [ ] CSS-Token-System (`aurora.css`) mit Light/Dark-Umschaltung
- [ ] CSS-Grid-Layout fuer 3-Panel-Ansicht
- [ ] Tauri-Fenster auf 1440x900 konfigurieren

Akzeptanzkriterien:
- App startet, zeigt 3-Panel-Layout mit Aurora-Light-Theme
- Datenbank wird beim Start erstellt, Schema-Version = 1
- Theme-Toggle zwischen hell/dunkel funktioniert

### Phase 2: Ordner & Dateien (Wochen 3–4)

**Ziel:** Ordner-Verwaltung und Datei-Import funktionieren.

Aufgaben:
- [ ] `commands/folders.rs` — CRUD fuer Ordner
- [ ] `commands/scanner.rs` — Directory-Scan mit `walkdir`, Fortschritts-Events
- [ ] `commands/files.rs` — Datei-CRUD, Suche, Format-Filter
- [ ] `Sidebar`-Komponente: Ordner-Liste, Auswahl, Datei-Zaehler
- [ ] `FileList`-Komponente: Mini-Cards, Suche, Format-Filter
- [ ] `SearchBar`- und `FilterChips`-Komponenten
- [ ] `FolderService` und `FileService` im Frontend

Akzeptanzkriterien:
- Ordner koennen erstellt, umbenannt und geloescht werden
- Verzeichnis-Scan erkennt .pes/.dst/.jef/.vp3-Dateien
- Dateien werden in der Datenbank registriert und in der Liste angezeigt
- Suche und Format-Filter funktionieren

### Phase 3: Stickformat-Parser (Wochen 5–7)

**Ziel:** PES-, DST-, JEF- und VP3-Dateien werden geparst.

Aufgaben:
- [ ] `EmbroideryParser`-Trait und Registry
- [ ] `parsers/pes.rs` — PES-Header, Stiche, Farben, eingebettetes Thumbnail
- [ ] `parsers/dst.rs` — Tajima-DST-Format, Stich-Befehle
- [ ] `parsers/jef.rs` — Janome-JEF-Format, Farbpalette
- [ ] `parsers/vp3.rs` — Viking-VP3-Format, Farbsektionen
- [ ] `services/thumbnail.rs` — Thumbnail-Generierung und Caching
- [ ] `MetadataPanel`-Komponente: Vorschau, Formular, Farb-Swatches

Akzeptanzkriterien:
- PES-Dateien: Abmessungen, Stichzahl, Farben, Thumbnail korrekt extrahiert
- DST-Dateien: Abmessungen und Stichzahl korrekt berechnet
- Thumbnails werden generiert und im MetadataPanel angezeigt
- Farb-Swatches mit Hex-Werten und Markennamen

### Phase 4: Metadaten & Tags (Wochen 8–9)

**Ziel:** Vollstaendiges Metadaten-Formular mit Tag-System.

Aufgaben:
- [ ] Metadaten-Formular: Name, Thema, Beschreibung, Lizenz (editierbar)
- [ ] Tag-System: Chip-Eingabe, Autocomplete, Tag-Erstellung
- [ ] Benutzerdefinierte Felder: Definition und Werte
- [ ] Speichern-Button: `update_file`-Command
- [ ] `Toolbar`-Komponente mit allen Aktions-Buttons
- [ ] `StatusBar`-Komponente mit Datei-Statistiken

Akzeptanzkriterien:
- Alle Metadatenfelder sind editierbar und speicherbar
- Tags koennen hinzugefuegt und entfernt werden
- Benutzerdefinierte Felder koennen definiert und befuellt werden
- Statusleiste zeigt korrekte Datei-Statistiken

### Phase 5: KI-Integration (Wochen 10–12)

**Ziel:** AI-Analyse ueber Ollama und OpenAI.

Aufgaben:
- [ ] `services/ai_client.rs` — HTTP-Client fuer Ollama/OpenAI Vision-API
- [ ] `commands/ai.rs` — Analyse-Command mit Bild + Prompt
- [ ] `AiPreviewDialog` — Prompt-Vorschau vor dem Senden
- [ ] `AiResultDialog` — Ergebnis-Review mit Accept/Reject pro Feld
- [ ] Farb-Erkennung: KI-generierte Farbnamen vs. Parser-Farben
- [ ] AI-Status-Badge in FileList und MetadataPanel
- [ ] `SettingsDialog` — KI-Tab: Provider, URL, API-Key, Modell, Temperatur

Akzeptanzkriterien:
- Ollama-Verbindung testbar (Statusanzeige)
- Einzeldatei-Analyse: Prompt wird angezeigt, Ergebnis kann reviewed werden
- Akzeptierte Ergebnisse werden in die Metadaten uebernommen
- "(KI-generiert)"-Label bei KI-befuellten Feldern

### Phase 6: Batch-Operationen & USB (Wochen 13–14)

**Ziel:** Batch-Rename, -Organize und USB-Export.

Aufgaben:
- [ ] `commands/batch.rs` — Rename mit Muster-Ersetzung, Organize mit Ordner-Erstellung
- [ ] `BatchDialog` — Fortschrittsanzeige mit Log
- [ ] USB-Export: Dateien auf USB-Geraet kopieren
- [ ] Batch-KI-Analyse: Mehrere Dateien sequenziell analysieren
- [ ] `SettingsDialog` — Dateiverwaltung-Tab: Rename/Organize-Muster

Akzeptanzkriterien:
- Batch-Rename benennt Dateien nach Muster um (z.B. `{name}_{theme}`)
- Batch-Organize verschiebt Dateien in Ordnerstruktur
- USB-Export kopiert ausgewaehlte Dateien auf Zielgeraet
- Fortschrittsanzeige mit Abbruch-Moeglichkeit

### Phase 7: Polish & Release (Wochen 15–16)

**Ziel:** Produktionsreife.

Aufgaben:
- [ ] `SettingsDialog` — Alle Tabs vollstaendig (Allgemein, Erscheinungsbild, KI, Dateiverwaltung, Benutzerdefiniert)
- [ ] Keyboard-Shortcuts (Cmd/Ctrl+S speichern, Cmd/Ctrl+F suchen, etc.)
- [ ] Dateisystem-Watcher (`notify`) — automatisches Erkennen neuer Dateien
- [ ] Splitter-Handles fuer Panel-Groessen-Anpassung
- [ ] Performance-Optimierung: virtuelles Scrolling fuer grosse Dateilisten
- [ ] Tauri-Bundle-Konfiguration fuer macOS, Windows, Linux
- [ ] App-Icon (`icon-windowed.icns` aus v1 uebernehmen)
- [ ] Fehlerbehandlung und Benutzer-Feedback (Toast-Benachrichtigungen)

Akzeptanzkriterien:
- Bundle-Groesse < 20 MB (macOS .dmg)
- Kaltstart < 1 s
- Alle Dialoge gemaess Design-Proposal implementiert
- Cross-Platform-Build erfolgreich (macOS + Linux mindestens)
- Keine bekannten kritischen Bugs

---

## 8. Migrationshinweise

### 8.1 Technologie-Mapping

| v1 (Python/Qt) | v2 (Rust/Tauri) | Anmerkungen |
|---|---|---|
| PySide6 Widgets | HTML/CSS Components | Komplett neue UI, kein 1:1-Mapping |
| Qt Stylesheets | CSS Custom Properties | Aurora-Tokens identisch, Anwendung anders |
| Python `sqlite3` | `rusqlite` (Backend) + `tauri-plugin-sql` (Frontend) | Neues Schema, keine Migration alter Daten |
| Pillow (PIL) | `image` Crate | Thumbnail-Generierung |
| NumPy | Nicht noetig | Stich-Berechnungen direkt in Rust |
| `requests` | `reqwest` | AI-API-Aufrufe |
| PyInstaller Bundle | Tauri Bundle | ~110 MB → ~15-20 MB |
| `QThread` | `tokio` async | Hintergrund-Operationen |
| Qt Signals/Slots | Tauri Events + EventBus | IPC-Kommunikation |
| `QFileSystemWatcher` | `notify` Crate | Dateisystem-Ueberwachung |
| `QSettings` | `settings`-Tabelle (SQLite) | Persistente Einstellungen |

### 8.2 Feature-Paritaetstabelle

| Feature | v1 Status | v2 Ziel | Phase |
|---|---|---|---|
| Ordner-Verwaltung | Vorhanden | Paritaet + verschachtelte Ordner | 2 |
| Datei-Import (Scan) | Vorhanden | Paritaet + Fortschrittsanzeige | 2 |
| PES-Parsing | Vorhanden | Paritaet (Rust-Reimplementierung) | 3 |
| DST-Parsing | Vorhanden | Paritaet | 3 |
| JEF-Parsing | Vorhanden | Paritaet | 3 |
| VP3-Parsing | Teilweise | Vollstaendig | 3 |
| Thumbnail-Vorschau | Vorhanden | Paritaet + Caching | 3 |
| Metadaten-Bearbeitung | Vorhanden | Paritaet + benutzerdefinierte Felder | 4 |
| Tag-System | Vorhanden | Paritaet + Autocomplete | 4 |
| KI-Analyse (Ollama) | Vorhanden | Paritaet + Prompt-Preview | 5 |
| KI-Analyse (OpenAI) | Vorhanden | Paritaet | 5 |
| Batch-Rename | Vorhanden | Paritaet + Muster-Historie | 6 |
| Batch-Organize | Vorhanden | Paritaet | 6 |
| USB-Export | Vorhanden | Paritaet | 6 |
| Dark Mode | Vorhanden | Paritaet (Aurora Dark) | 1 |
| Suche | Vorhanden | Paritaet + Format-Filter | 2 |
| Dateisystem-Watcher | Nicht vorhanden | Neu | 7 |
| Cross-Platform | Nur macOS | macOS + Windows + Linux | 7 |
| Virtuelle Scrolling | Nicht vorhanden | Neu (fuer grosse Listen) | 7 |

### 8.3 Risikomatrix

| Risiko | Wahrscheinlichkeit | Auswirkung | Mitigation |
|---|---|---|---|
| PES-Parser-Bugs (komplexes Binaerformat) | Mittel | Hoch | Testdateien aus v1, Unit-Tests pro Format-Variante |
| AI-API-Inkompatibilitaeten (Ollama-Versionen) | Niedrig | Mittel | Abstraktes Provider-Interface, Verbindungstest |
| CSS-Rendering-Unterschiede (WebView-Versionen) | Niedrig | Niedrig | WebView2 (Windows) / WebKit (macOS/Linux) testen |
| Performance bei >1000 Dateien | Mittel | Mittel | Virtuelles Scrolling, paginierte DB-Queries |
| Theme-Token-Inkonsistenzen | Niedrig | Niedrig | Design-Proposal als Single Source of Truth |
| Tauri v2 Breaking Changes | Niedrig | Hoch | Abhaengigkeitsversionen pinnen, Changelog verfolgen |

### 8.4 Nicht migrierte Elemente

Die folgenden v1-Aspekte werden in v2 bewusst nicht uebernommen:

- **Python-Quellcode**: Der Code ist im PyInstaller-Bundle kompiliert und nicht extrahierbar. Die Funktionalitaet wird anhand der App-Nutzung und des Design-Proposals nachgebaut.
- **Qt-Stylesheets**: Werden durch CSS Custom Properties ersetzt. Die Farbwerte sind identisch (Aurora-Palette), aber die Anwendung ist grundlegend anders.
- **NumPy-Abhaengigkeit**: Stich-Berechnungen (Abmessungen, Stichzahl) werden direkt in Rust implementiert — effizienter und ohne externe Abhaengigkeit.
- **macOS-spezifische Features**: Traffic-Light-Buttons und andere macOS-Widgets werden durch Tauri-Dekorationen ersetzt.

---

*Ende des technischen Proposals.*
*Naechster Schritt: Beginn mit Phase 1 (Fundament) im Release-Zyklus `26.03-a1`.*

# StichMan v2 — Sprint-Plan

> Basierend auf dem technischen Proposal `release_26.03-a1/init_01.md`
> Autor: CarpeAsrael · Datum: 2026-03-08
> Release-Zyklus: `26.03-a1`

---

## Uebersicht

| Sprint | Name | Wochen | Phase (Proposal) | Tickets |
|--------|------|--------|-------------------|---------|
| 1 | Fundament Backend | 1 | Phase 1 | 5 |
| 2 | Fundament Frontend | 2 | Phase 1 | 7 |
| 3 | Ordner-Verwaltung | 3 | Phase 2 | 4 |
| 4 | Datei-Import & Liste | 4 | Phase 2 | 6 |
| 5 | PES- & DST-Parser | 5–6 | Phase 3 | 7 |
| 6 | Erweiterte Parser & Thumbnails | 7 | Phase 3 | 4 |
| 7 | Metadaten, Tags & KI-Vorbereitung | 8–9 | Phase 4 | 7 |
| 8 | KI-Integration | 10–12 | Phase 5 | 7 |
| 9 | Batch-Operationen & USB-Export | 13–14 | Phase 6 | 6 |
| 10 | Polish & Release | 15–16 | Phase 7 | 8 |
| | | **16 Wochen** | | **61** |

---

## Definition of Done (pro Ticket)

Gemaess `CLAUDE.md`:

1. **Analyse-Agent** → Dokument in `docs/analysis/<yyyymmdd>_<counter>_<kurzname>.md`
2. **Vollstaendige Implementierung** gemaess Analyse-Ergebnis
3. **4 Review-Agenten** (2× Codex + 2× Claude) mit **0 Findings**
4. **Linting, Typecheck, Tests** bestanden (`npm run build`, `cargo check`, `cargo test`)
5. **Commit** dokumentiert, Ticket geschlossen

---

## Sprint 1 — Fundament Backend (Woche 1)

**Ziel:** Rust-Backend-Grundstruktur steht, Datenbank wird beim Start erstellt.

**Abhaengigkeiten:** Keine (erster Sprint)

### S1-T1: Rust-Modulstruktur aufsetzen

**Beschreibung:** Die Verzeichnisstruktur unter `src-tauri/src/` gemaess Proposal §2.4 anlegen.

**Dateien:**
- `src-tauri/src/db/mod.rs` — DB-Modul (leer, `pub mod migrations; pub mod models;`)
- `src-tauri/src/db/migrations.rs` — Platzhalter
- `src-tauri/src/db/models.rs` — Platzhalter
- `src-tauri/src/commands/mod.rs` — Command-Registrierung (leer)
- `src-tauri/src/parsers/mod.rs` — Parser-Trait-Platzhalter
- `src-tauri/src/services/mod.rs` — Services-Platzhalter
- `src-tauri/src/error.rs` — Platzhalter
- `src-tauri/src/lib.rs` — Module-Deklarationen ergaenzen

**Akzeptanzkriterien:**
- [ ] `cargo check` kompiliert erfolgreich
- [ ] Alle Module sind in `lib.rs` deklariert
- [ ] Verzeichnisstruktur entspricht Proposal §2.4

---

### S1-T2: AppError-Typ und Serialisierung

**Beschreibung:** Zentraler Fehlertyp `AppError` gemaess Proposal §4.7 implementieren.

**Dateien:**
- `src-tauri/src/error.rs`

**Implementierung (gemaess Proposal):**
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
```

**Akzeptanzkriterien:**
- [ ] `AppError` implementiert `thiserror::Error`
- [ ] `AppError` implementiert `serde::Serialize` (fuer Tauri-IPC)
- [ ] Alle 6 Varianten vorhanden: Database, Io, Parse, Ai, NotFound, Validation
- [ ] `cargo check` kompiliert

---

### S1-T3: Cargo-Dependencies ergaenzen

**Beschreibung:** Alle im Proposal §4.1 genannten Crates zum `Cargo.toml` hinzufuegen.

**Dateien:**
- `src-tauri/Cargo.toml`

**Neue Dependencies:**
- `rusqlite = { version = "0.31", features = ["bundled"] }`
- `tokio = { version = "1", features = ["full"] }`
- `reqwest = { version = "0.12", features = ["json", "multipart"] }`
- `notify = "6"`
- `image = "0.25"`
- `walkdir = "2"`
- `chrono = { version = "0.4", features = ["serde"] }`
- `sha2 = "0.10"`
- `base64 = "0.22"`
- `uuid = { version = "1", features = ["v4"] }`
- `thiserror = "1"`
- `byteorder = "1"`

**Akzeptanzkriterien:**
- [ ] Alle 12 neuen Crates in `Cargo.toml`
- [ ] `cargo check` kompiliert erfolgreich (keine Versionskonflikte)

---

### S1-T4: SQLite-Schema implementieren (10 Tabellen)

**Beschreibung:** Alle 10 Datenbanktabellen gemaess Proposal §3 implementieren, inkl. Migrationslogik ueber `schema_version`.

**Dateien:**
- `src-tauri/src/db/migrations.rs` — `run_migrations(conn: &Connection)` Funktion
- `src-tauri/src/db/mod.rs` — Connection-Setup, `init_database()` Funktion
- `src-tauri/src/db/models.rs` — Rust-Structs fuer alle Tabellen

**Tabellen (gemaess Proposal §3.1–§3.9):**
1. `schema_version` — Migrationsversion
2. `folders` — Ordner-Eintraege
3. `embroidery_files` — Stickdateien mit Metadaten
4. `file_formats` — Format-Zuordnungen
5. `file_thread_colors` — Fadenfarben
6. `tags` — Tag-Definitionen
7. `file_tags` — n:m Datei-Tag-Zuordnung
8. `ai_analysis_results` — KI-Analyse-Ergebnisse
9. `settings` — Key-Value-Einstellungen (inkl. Default-Werte)
10. `custom_field_definitions` + `custom_field_values` — Benutzerdefinierte Felder

**Rust-Structs (models.rs):**
- `Folder`, `EmbroideryFile`, `FileFormat`, `ThreadColor`, `Tag`, `AiAnalysisResult`, `Setting`, `CustomFieldDef`, `CustomFieldValue`
- Alle mit `#[derive(Debug, Clone, Serialize, Deserialize)]`

**Akzeptanzkriterien:**
- [ ] `init_database()` erstellt alle 10+ Tabellen
- [ ] `schema_version` wird auf `1` gesetzt
- [ ] Default-Settings werden eingefuegt (library_root, theme_mode, ai_provider, etc.)
- [ ] Alle Indizes gemaess Proposal angelegt
- [ ] Rust-Structs matchen die DB-Spalten
- [ ] `cargo test` — Migrations-Test: DB anlegen, Schema pruefen, erneut ausfuehren (idempotent)

---

### S1-T5: Tauri-Fensterkonfiguration

**Beschreibung:** Tauri-Fenster gemaess Proposal §6.3 konfigurieren.

**Dateien:**
- `src-tauri/tauri.conf.json`

**Konfiguration:**
- Titel: `"StichMan"`
- Groesse: 1440 × 900
- Mindestgroesse: 960 × 640
- `resizable: true`, `decorations: true`, `fullscreen: false`

**Integration:**
- `lib.rs`: Datenbank-Initialisierung beim App-Start einbinden (`init_database()` im `setup`-Hook)

**Akzeptanzkriterien:**
- [ ] App startet mit korrektem Fenstertitel "StichMan"
- [ ] Fenster hat 1440×900 als Startgroesse
- [ ] Fenster kann nicht kleiner als 960×640 gemacht werden
- [ ] Datenbank wird beim Start erstellt (`stitch_manager.db`)

---

## Sprint 2 — Fundament Frontend (Woche 2)

**Ziel:** Frontend-Grundgeruest mit Component-System, State-Management, Aurora-Theme und 3-Panel-Layout.

**Abhaengigkeiten:** Sprint 1 (Backend-Grundstruktur muss stehen)

### S2-T1: TypeScript-Typen definieren

**Beschreibung:** Shared TypeScript-Interfaces gemaess Proposal §2.4 und DB-Schema.

**Dateien:**
- `src/types/index.ts`

**Interfaces:**
```typescript
interface Folder { id: number; name: string; path: string; parentId: number | null; sortOrder: number; createdAt: string; updatedAt: string; }
interface EmbroideryFile { id: number; folderId: number; filename: string; filepath: string; name: string | null; theme: string | null; description: string | null; license: string | null; widthMm: number | null; heightMm: number | null; stitchCount: number | null; colorCount: number | null; fileSizeBytes: number | null; thumbnailPath: string | null; aiAnalyzed: boolean; aiConfirmed: boolean; createdAt: string; updatedAt: string; }
interface FileFormat { id: number; fileId: number; format: string; formatVersion: string | null; filepath: string; fileSizeBytes: number | null; parsed: boolean; }
interface ThreadColor { id: number; fileId: number; sortOrder: number; colorHex: string; colorName: string | null; brand: string | null; brandCode: string | null; isAi: boolean; }
interface Tag { id: number; name: string; createdAt: string; }
interface AiAnalysisResult { id: number; fileId: number; provider: string; model: string; parsedName: string | null; parsedTheme: string | null; parsedDesc: string | null; parsedTags: string[] | null; parsedColors: string[] | null; accepted: boolean; analyzedAt: string; }
interface FileUpdate { name?: string; theme?: string; description?: string; license?: string; }
type ThemeMode = 'hell' | 'dunkel';
```

**Akzeptanzkriterien:**
- [ ] Alle Interfaces matchen die Rust-Structs / DB-Schema
- [ ] `npm run build` kompiliert ohne TypeScript-Fehler

---

### S2-T2: Component-Basisklasse

**Beschreibung:** Leichtgewichtige Basisklasse fuer alle UI-Komponenten gemaess Proposal §5.1.

**Dateien:**
- `src/components/Component.ts`

**Implementierung:**
- `abstract render(): void`
- `subscribe(event, handler)` mit automatischem Cleanup
- `destroy()` raeumt Subscriptions und DOM auf

**Akzeptanzkriterien:**
- [ ] Abstrakte Klasse mit `render()`, `subscribe()`, `destroy()`
- [ ] Subscriptions werden bei `destroy()` automatisch abgemeldet
- [ ] TypeScript kompiliert

---

### S2-T3: AppState (Reaktiver State-Store)

**Beschreibung:** Zentraler State-Store gemaess Proposal §5.2.

**Dateien:**
- `src/state/AppState.ts`

**State-Interface:**
```typescript
interface State {
    folders: Folder[];
    selectedFolderId: number | null;
    files: EmbroideryFile[];
    selectedFileId: number | null;
    searchQuery: string;
    formatFilter: string | null;
    settings: Record<string, string>;
    theme: ThemeMode;
}
```

**Methoden:** `get(key)`, `set(key, value)`, `on(key, listener) → unsubscribe`

**Akzeptanzkriterien:**
- [ ] State-Aenderungen via `set()` benachrichtigen alle Listener
- [ ] `on()` gibt eine Unsubscribe-Funktion zurueck
- [ ] Initiale State-Werte sind gesetzt (leere Arrays, null-Selektionen)

---

### S2-T4: EventBus

**Beschreibung:** Frontend-interner Event-Bus gemaess Proposal §5.3 mit Tauri-Event-Bridging.

**Dateien:**
- `src/state/EventBus.ts`

**Methoden:** `emit(event, data?)`, `on(event, handler) → unsubscribe`

**Tauri-Bridge:**
```typescript
listen('scan:progress', (e) => EventBus.emit('scan:progress', e.payload));
listen('ai:complete',   (e) => EventBus.emit('ai:complete',   e.payload));
listen('batch:progress', (e) => EventBus.emit('batch:progress', e.payload));
```

**Akzeptanzkriterien:**
- [ ] `emit()` ruft alle registrierten Handler auf
- [ ] `on()` gibt Unsubscribe-Funktion zurueck
- [ ] Tauri-Backend-Events werden an den Frontend-Bus weitergeleitet

---

### S2-T5: Aurora CSS-Tokens

**Beschreibung:** Design-Token-System gemaess Proposal §6.1 als CSS Custom Properties.

**Dateien:**
- `src/styles/aurora.css`

**Inhalt:**
- `:root` / `[data-theme="hell"]` — alle Light-Theme-Tokens (Farben, Fonts, Spacing, Radius, Shadows)
- `[data-theme="dunkel"]` — Dark-Theme-Overrides
- Genaue Werte aus Proposal §6.1

**Akzeptanzkriterien:**
- [ ] Alle 30+ CSS Custom Properties definiert (--color-*, --font-*, --spacing-*, --radius-*, --shadow-*)
- [ ] Light- und Dark-Theme vollstaendig
- [ ] Font-Family: "Helvetica Neue", "Segoe UI", Helvetica, Arial, sans-serif

---

### S2-T6: CSS-Grid-Layout (3-Panel-Ansicht)

**Beschreibung:** Haupt-Layout gemaess Proposal §5.7.

**Dateien:**
- `src/styles/layout.css`
- `index.html` — Grid-Container-Markup

**Grid-Definition:**
```css
grid-template-rows: 28px 48px 1fr 22px;
grid-template-columns: var(--sidebar-width, 240px) var(--center-width, 480px) 1fr;
grid-template-areas:
    "menu    menu    menu"
    "toolbar toolbar toolbar"
    "sidebar center  right"
    "status  status  status";
```

**Akzeptanzkriterien:**
- [ ] 4 Zeilen: Menue, Toolbar, Hauptbereich (3 Spalten), Statusleiste
- [ ] 3 Spalten im Hauptbereich: Sidebar, Center, Right
- [ ] `height: 100vh; overflow: hidden;`
- [ ] Placeholder-Inhalte in jedem Grid-Bereich sichtbar

---

### S2-T7: Theme-Toggle (hell/dunkel)

**Beschreibung:** Theme-Umschaltung zwischen hell und dunkel ueber `data-theme`-Attribut.

**Dateien:**
- `src/main.ts` — Theme-Initialisierung beim App-Start
- `index.html` — `data-theme="hell"` als Default auf `<html>`

**Logik:**
1. Beim Start: `theme_mode` aus Settings-Tabelle laden (via `tauri-plugin-sql`)
2. `<html data-theme="hell|dunkel">` setzen
3. Toggle-Funktion: Theme wechseln, in DB speichern, Attribut aktualisieren

**Akzeptanzkriterien:**
- [ ] App startet mit dem gespeicherten Theme (Default: "hell")
- [ ] Theme-Toggle aendert `data-theme`-Attribut
- [ ] Alle Aurora-CSS-Tokens reagieren auf den Theme-Wechsel
- [ ] Theme-Wahl wird in der Datenbank persistiert

---

## Sprint 3 — Ordner-Verwaltung (Woche 3)

**Ziel:** Ordner koennen erstellt, angezeigt, umbenannt und geloescht werden.

**Abhaengigkeiten:** Sprint 1 (DB-Schema), Sprint 2 (Component-System, AppState)

### S3-T1: commands/folders.rs

**Beschreibung:** Tauri-Commands fuer Ordner-CRUD gemaess Proposal §4.2.2.

**Dateien:**
- `src-tauri/src/commands/folders.rs`
- `src-tauri/src/commands/mod.rs` — Registrierung
- `src-tauri/src/lib.rs` — `invoke_handler` ergaenzen

**Commands:**
- `get_folders() → Vec<Folder>`
- `create_folder(name, path, parent_id?) → Folder`
- `update_folder(folder_id, name?) → Folder`
- `delete_folder(folder_id) → ()`
- `get_folder_file_count(folder_id) → i64`

**Akzeptanzkriterien:**
- [ ] Alle 5 Commands implementiert und registriert
- [ ] `create_folder` validiert: Name nicht leer, Pfad existiert
- [ ] `delete_folder` loescht kaskadierend (Dateien werden mitgeloescht)
- [ ] `get_folders` gibt hierarchische Struktur zurueck (parent_id)
- [ ] `cargo test` — CRUD-Zyklus mit In-Memory-DB

---

### S3-T2: FolderService (Frontend)

**Beschreibung:** Frontend-Service als `invoke()`-Wrapper fuer Ordner-Commands.

**Dateien:**
- `src/services/FolderService.ts`

**Methoden:**
- `getAll(): Promise<Folder[]>`
- `create(name, path, parentId?): Promise<Folder>`
- `update(folderId, name): Promise<Folder>`
- `remove(folderId): Promise<void>`
- `getFileCount(folderId): Promise<number>`

**Akzeptanzkriterien:**
- [ ] Alle 5 Methoden implementiert
- [ ] Parameter-Mapping korrekt (camelCase → snake_case fuer Tauri)
- [ ] TypeScript kompiliert

---

### S3-T3: Sidebar-Komponente

**Beschreibung:** Ordner-Navigation im linken Panel gemaess Proposal §5.5.

**Dateien:**
- `src/components/Sidebar.ts`
- `src/styles/components.css` — Sidebar-Styles

**Funktionen:**
- Ordner-Liste anzeigen (Name + Datei-Zaehler)
- Ordner auswaehlen → `AppState.set('selectedFolderId', id)`
- "+ Neuer Ordner"-Button → Ordner erstellen (per Dialog oder Inline-Eingabe)
- Aktiver Ordner visuell hervorgehoben

**State-Bindings:** `folders`, `selectedFolderId`

**Akzeptanzkriterien:**
- [ ] Ordner werden als Liste im linken Panel dargestellt
- [ ] Klick auf Ordner setzt `selectedFolderId`
- [ ] Aktiver Ordner hat visuelles Highlight (accent-Farbe)
- [ ] Datei-Zaehler pro Ordner wird angezeigt
- [ ] Neuer Ordner kann erstellt werden

---

### S3-T4: Tauri-Permissions fuer Folder-Commands

**Beschreibung:** Permissions fuer die neuen Commands in der Capabilities-Datei eintragen.

**Dateien:**
- `src-tauri/capabilities/default.json`

**Aenderungen:**
- Custom-Permission-String fuer die Folder-Commands hinzufuegen (oder `"core:default"` pruefen)
- Sicherstellen, dass `invoke()` aus dem Frontend die Commands erreichen kann

**Akzeptanzkriterien:**
- [ ] Frontend kann `invoke('get_folders')` erfolgreich aufrufen
- [ ] Keine Permission-Fehler in der Konsole

---

## Sprint 4 — Datei-Import & Liste (Woche 4)

**Ziel:** Verzeichnis-Scan, Datei-Import und Dateiliste funktionieren.

**Abhaengigkeiten:** Sprint 3 (Ordner-Verwaltung)

### S4-T1: commands/scanner.rs

**Beschreibung:** Directory-Scanner mit `walkdir` und Fortschritts-Events gemaess Proposal §4.2.3.

**Dateien:**
- `src-tauri/src/commands/scanner.rs`
- `src-tauri/src/commands/mod.rs` — Registrierung

**Commands:**
- `scan_directory(path, app_handle) → ScanResult` — Emittiert `scan:progress`, `scan:file-found`, `scan:complete`
- `import_files(file_paths, folder_id) → Vec<EmbroideryFile>`

**Logik:**
1. Rekursiver Walk mit `walkdir`
2. Dateiendungen filtern: `.pes`, `.dst`, `.jef`, `.vp3`
3. Pro gefundener Datei: Event emittieren
4. `import_files`: DB-Eintraege fuer jede Datei erstellen (filename, filepath, file_size_bytes)

**Akzeptanzkriterien:**
- [ ] Scanner findet alle .pes/.dst/.jef/.vp3-Dateien rekursiv
- [ ] Fortschritts-Events werden emittiert
- [ ] `import_files` erstellt DB-Eintraege mit korrekter folder_id
- [ ] Duplikate werden erkannt (filepath UNIQUE Constraint)
- [ ] `cargo test` — Scan-Test mit Temp-Verzeichnis

---

### S4-T2: commands/files.rs (Lese-Operationen)

**Beschreibung:** Datei-Abfrage-Commands gemaess Proposal §4.2.1 (nur Lesen in diesem Sprint).

**Dateien:**
- `src-tauri/src/commands/files.rs`
- `src-tauri/src/commands/mod.rs` — Registrierung

**Commands:**
- `get_files(folder_id?, search?, format_filter?) → Vec<EmbroideryFile>`
- `get_file(file_id) → EmbroideryFile`
- `get_file_formats(file_id) → Vec<FileFormat>`
- `get_file_colors(file_id) → Vec<ThreadColor>`
- `get_file_tags(file_id) → Vec<Tag>`

**Akzeptanzkriterien:**
- [ ] `get_files` unterstuetzt Filterung nach folder_id, Suche (LIKE auf name/filename), format_filter
- [ ] `get_file` gibt 404/NotFound zurueck bei ungueltigem ID
- [ ] Alle 5 Commands implementiert und registriert
- [ ] `cargo test` — Query-Tests mit vorbereiteten Daten

---

### S4-T3: FileService und ScannerService (Frontend)

**Beschreibung:** Frontend-Services als `invoke()`-Wrapper.

**Dateien:**
- `src/services/FileService.ts`
- `src/services/ScannerService.ts`

**FileService-Methoden:**
- `getFiles(folderId?, search?, formatFilter?): Promise<EmbroideryFile[]>`
- `getFile(fileId): Promise<EmbroideryFile>`
- `getFormats(fileId): Promise<FileFormat[]>`
- `getColors(fileId): Promise<ThreadColor[]>`
- `getTags(fileId): Promise<Tag[]>`

**ScannerService-Methoden:**
- `scanDirectory(path): Promise<ScanResult>`
- `importFiles(filePaths, folderId): Promise<EmbroideryFile[]>`

**Akzeptanzkriterien:**
- [ ] Alle Methoden implementiert
- [ ] TypeScript kompiliert

---

### S4-T4: SearchBar-Komponente

**Beschreibung:** Such-Eingabefeld mit Debounced Input gemaess Proposal §5.5.

**Dateien:**
- `src/components/SearchBar.ts`
- `src/styles/components.css` — SearchBar-Styles

**Funktionen:**
- Textfeld mit Suchsymbol
- Debounce (300ms): Eingabe → `AppState.set('searchQuery', value)`
- Clear-Button zum Zuruecksetzen

**State-Bindings:** `searchQuery`

**Akzeptanzkriterien:**
- [ ] Eingabe setzt `searchQuery` mit 300ms Debounce
- [ ] Clear-Button setzt Suche zurueck
- [ ] Styling gemaess Aurora-Tokens

---

### S4-T5: FilterChips-Komponente

**Beschreibung:** Format-Filter als klickbare Chips gemaess Proposal §5.5.

**Dateien:**
- `src/components/FilterChips.ts`
- `src/styles/components.css` — Chip-Styles

**Chips:** Alle, PES, DST, JEF, VP3

**Logik:** Klick auf Chip → `AppState.set('formatFilter', 'PES' | null)`

**State-Bindings:** `formatFilter`

**Akzeptanzkriterien:**
- [ ] 5 Chips angezeigt: Alle, PES, DST, JEF, VP3
- [ ] Aktiver Filter visuell hervorgehoben (accent-Farbe)
- [ ] Klick auf "Alle" setzt Filter auf `null`
- [ ] Styling mit `--radius-pill`

---

### S4-T6: FileList-Komponente

**Beschreibung:** Dateiliste mit Mini-Cards gemaess Proposal §5.5 und §6.2.

**Dateien:**
- `src/components/FileList.ts`
- `src/styles/components.css` — FileList- und File-Card-Styles

**Funktionen:**
- Mini-Cards: Thumbnail-Platzhalter, Dateiname, Format-Chip(s)
- Klick auf Karte → `AppState.set('selectedFileId', id)`
- Reagiert auf State-Aenderungen: `selectedFolderId`, `searchQuery`, `formatFilter`
- Bei Aenderung: `FileService.getFiles()` aufrufen und Liste neu rendern

**Card-Layout (gemaess Proposal §6.2):**
```css
.file-card { display: flex; align-items: center; gap: var(--spacing-3); padding: var(--spacing-3); height: 72px; }
```

**State-Bindings:** `files`, `selectedFileId`

**Akzeptanzkriterien:**
- [ ] Dateien werden als Mini-Cards dargestellt
- [ ] Selektierte Karte hat accent-Border und accent-Background
- [ ] Aenderung von Ordner/Suche/Filter laedt Dateien neu
- [ ] Leerer Zustand zeigt Platzhalter-Nachricht

---

## Sprint 5 — PES- & DST-Parser (Wochen 5–6)

**Ziel:** PES- und DST-Dateien werden vollstaendig geparst.

**Abhaengigkeiten:** Sprint 1 (Modulstruktur, AppError)

### S5-T1: EmbroideryParser-Trait und Registry

**Beschreibung:** Parser-Trait gemaess Proposal §4.3 und eine Registry zur Format-Erkennung.

**Dateien:**
- `src-tauri/src/parsers/mod.rs`

**Trait:**
```rust
pub trait EmbroideryParser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn parse(&self, data: &[u8]) -> Result<ParsedFileInfo, AppError>;
    fn extract_thumbnail(&self, data: &[u8]) -> Result<Option<Vec<u8>>, AppError>;
}
```

**Structs:** `ParsedFileInfo`, `ParsedColor` (gemaess Proposal §4.3)

**Registry:** `get_parser(extension: &str) → Option<&dyn EmbroideryParser>`

**Akzeptanzkriterien:**
- [ ] Trait definiert mit 3 Methoden
- [ ] `ParsedFileInfo` und `ParsedColor` mit Serde-Derives
- [ ] Registry-Funktion kann Parser nach Dateiendung nachschlagen
- [ ] `cargo check` kompiliert

---

### S5-T2: PES-Parser — Header und Farbobjekte

**Beschreibung:** PES-Datei-Header parsen gemaess Proposal §4.4.2.

**Dateien:**
- `src-tauri/src/parsers/pes.rs`

**Implementierung:**
1. Magic-Byte-Pruefung: `#PES`
2. Version lesen: `0060` (v6.0)
3. PEC-Offset lesen (uint32 LE bei Offset 8)
4. Design-Name extrahieren (Offset 16: Laenge + String)
5. PES-Farbobjekte parsen:
   - Code (ASCII), RGB (3 Bytes), Farbname, Markenname
   - Verifizierte Palette gemaess Proposal-Tabelle

**Akzeptanzkriterien:**
- [ ] Magic `#PES` und Version `0060` korrekt erkannt
- [ ] PEC-Offset korrekt gelesen
- [ ] Farbobjekte mit Code, RGB, Name, Marke extrahiert
- [ ] `cargo test` — Test mit Beispieldateien aus `example files/`

---

### S5-T3: PES-Parser — PEC-Sektion und Stich-Dekodierung

**Beschreibung:** PEC-Header und Stich-Daten parsen gemaess Proposal §4.4.2.

**Dateien:**
- `src-tauri/src/parsers/pes.rs` (Erweiterung)

**Implementierung:**
1. PEC-Header (512 Bytes): Label, Farbanzahl (Offset 48), Palettenindizes
2. Grafik-Header (PEC+512, 20 Bytes): Stich-Datenlaenge, Designbreite/-hoehe (uint16 LE × 0.1mm)
3. Stich-Dekodierung (ab PEC+532):
   - Kurzform (1 Byte, Bit7=0): 0x00–0x3F positiv, 0x40–0x7F negativ (7-Bit-Zweierkomplement)
   - Langform (2 Bytes, Bit7=1): 12-Bit Verschiebung, Jump/Trim-Flag (Bit5)
   - **KRITISCH:** Farbwechsel = 3 Bytes (`FE B0 XX`), nicht 2!
4. End-Marker: `0xFF`

**Akzeptanzkriterien:**
- [ ] Designbreite und -hoehe korrekt berechnet (×0.1mm)
- [ ] Stichzahl durch Dekodierung ermittelt
- [ ] Farbwechsel als 3-Byte-Sequenz behandelt (`FE B0 XX`)
- [ ] Normal-, Jump- und Trim-Stiche unterschieden
- [ ] `cargo test` — Stichzahl und Dimensionen gegen Referenzwerte pruefen

---

### S5-T4: PES-Parser — Thumbnail-Extraktion

**Beschreibung:** Eingebettetes Monochrom-Thumbnail aus PEC-Sektion extrahieren.

**Dateien:**
- `src-tauri/src/parsers/pes.rs` (Erweiterung)

**Implementierung (gemaess Proposal §4.4.2):**
- Position: `PEC-Offset + 532 + Stich-Datenlaenge`
- Erstes Bild: 48 × 38 Pixel, 1 Bit/Pixel, MSB-first
- 228 Bytes pro Thumbnail
- Konvertierung in Grayscale-Pixel-Array

**Akzeptanzkriterien:**
- [ ] Thumbnail-Bytes korrekt lokalisiert
- [ ] 48×38 Monochrom-Bild korrekt dekodiert
- [ ] `extract_thumbnail()` gibt `Some(Vec<u8>)` zurueck
- [ ] `cargo test` — Thumbnail ist nicht komplett schwarz/weiss (hat Muster)

---

### S5-T5: DST-Parser — Header

**Beschreibung:** Tajima-DST 512-Byte-Header parsen gemaess Proposal §4.4.1.

**Dateien:**
- `src-tauri/src/parsers/dst.rs`

**Header-Felder:**
| Offset | Label | Beschreibung |
|--------|-------|--------------|
| 0 | `LA:` | Design-Label (16 Zeichen) |
| 20 | `ST:` | Stichzahl |
| 31 | `CO:` | Farbwechsel-Anzahl |
| 38 | `+X:` | Max positive X (0.1mm) |
| 47 | `-X:` | Max negative X (0.1mm) |
| 56 | `+Y:` | Max positive Y (0.1mm) |
| 65 | `-Y:` | Max negative Y (0.1mm) |

**Dimensionsberechnung:** `Breite = (+X + -X) × 0.1mm`, `Hoehe = (+Y + -Y) × 0.1mm`

**Akzeptanzkriterien:**
- [ ] Alle Header-Felder korrekt extrahiert
- [ ] Dimensionen korrekt berechnet
- [ ] `CO:` Feld korrekt geparst (0 = 1 Farbe, N = N+1 Farben)
- [ ] `cargo test` — Header-Werte gegen bekannte Testdateien pruefen

---

### S5-T6: DST-Parser — Stich-Dekodierung

**Beschreibung:** Balanced-Ternary-Stich-Dekodierung gemaess Proposal §4.4.1.

**Dateien:**
- `src-tauri/src/parsers/dst.rs` (Erweiterung)

**Implementierung:**
- 3-Byte-Triplets ab Offset 512
- Balanced-Ternary mit Gewichten 1, 3, 9, 27, 81
- Befehlstypen: Normal (0x03), Jump (0x83), Color Change (0xC3), End (0xF3)
- `decode_dst_triplet(b0, b1, b2) → (dx, dy)` gemaess Proposal

**DST-Einschraenkungen:**
- Keine Farbinformationen (nur Wechselanzahl)
- Kein eingebettetes Thumbnail
- `extract_thumbnail()` gibt `None` zurueck

**Akzeptanzkriterien:**
- [ ] Triplet-Dekodierung ergibt korrekte dx/dy-Werte
- [ ] Kumulative Ausdehnung stimmt mit Header-Werten (+X/-X/+Y/-Y) ueberein
- [ ] Befehlstypen korrekt erkannt (Normal, Jump, Color Change, End)
- [ ] Stichzahl = Anzahl dekodierter Triplets (ohne End)
- [ ] `cargo test` — Validierung gegen alle DST-Testdateien

---

### S5-T7: parse_embroidery_file Command

**Beschreibung:** Tauri-Command zum Parsen einer einzelnen Stickdatei gemaess Proposal §4.2.3.

**Dateien:**
- `src-tauri/src/commands/scanner.rs` (Erweiterung)
- `src-tauri/src/lib.rs` — Command registrieren

**Command:**
```rust
#[tauri::command]
async fn parse_embroidery_file(filepath: String) -> Result<ParsedFileInfo, AppError>
```

**Logik:**
1. Dateiendung ermitteln
2. Parser aus Registry holen
3. Datei lesen, `parser.parse(data)` aufrufen
4. `ParsedFileInfo` zurueckgeben

**Akzeptanzkriterien:**
- [ ] Command nimmt Dateipfad entgegen und gibt ParsedFileInfo zurueck
- [ ] Unbekannte Formate ergeben `AppError::Parse`
- [ ] Permissions in `default.json` konfiguriert
- [ ] `cargo test` — Parse-Test mit PES- und DST-Datei

---

## Sprint 6 — Erweiterte Parser & Thumbnails (Woche 7)

**Ziel:** JEF- und VP3-Parser, Thumbnail-Generierung und MetadataPanel.

**Abhaengigkeiten:** Sprint 5 (Parser-Trait, PES/DST-Parser)

### S6-T1: JEF-Parser

**Beschreibung:** Janome-JEF-Format-Parser gemaess Proposal §4.3.

**Dateien:**
- `src-tauri/src/parsers/jef.rs`
- `src-tauri/src/parsers/mod.rs` — Registry ergaenzen

**Implementierung:**
- JEF-Header parsen (Stichzahl, Dimensionen, Farbanzahl)
- Janome-spezifische Farbpalette
- Stich-Daten dekodieren
- `extract_thumbnail()` → `None` (JEF hat kein eingebettetes Thumbnail)

**Akzeptanzkriterien:**
- [ ] `EmbroideryParser`-Trait implementiert
- [ ] Stichzahl, Dimensionen, Farbanzahl korrekt extrahiert
- [ ] Farbpalette mit Janome-Farben
- [ ] `cargo test` — Parse-Test mit JEF-Datei (falls Testdateien vorhanden)

---

### S6-T2: VP3-Parser

**Beschreibung:** Viking/Pfaff-VP3-Format-Parser gemaess Proposal §4.3.

**Dateien:**
- `src-tauri/src/parsers/vp3.rs`
- `src-tauri/src/parsers/mod.rs` — Registry ergaenzen

**Implementierung:**
- VP3-Header parsen
- Komplexe Farbsektionen dekodieren
- Stich-Daten lesen
- `extract_thumbnail()` → `None`

**Akzeptanzkriterien:**
- [ ] `EmbroideryParser`-Trait implementiert
- [ ] Stichzahl, Dimensionen, Farbanzahl korrekt extrahiert
- [ ] Farbsektionen korrekt dekodiert
- [ ] `cargo test` — Parse-Test mit VP3-Datei (falls Testdateien vorhanden)

---

### S6-T3: ThumbnailGenerator

**Beschreibung:** Thumbnail-Generierung und Caching gemaess Proposal §4.6.

**Dateien:**
- `src-tauri/src/services/thumbnail.rs`
- `src-tauri/src/services/mod.rs`

**Implementierung:**
```rust
pub struct ThumbnailGenerator {
    cache_dir: PathBuf,       // {metadata_root}/thumbnails/
    target_size: (u32, u32),  // 192 x 192 px
}
```

**Strategie (gemaess Proposal §4.6):**
1. **PES:** Eingebettetes 48×38 Monochrom-Thumbnail extrahieren, auf 192×192 skalieren. Optional: Farbiges Thumbnail aus Stich-Koordinaten rendern.
2. **DST:** Stich-Koordinaten aus Balanced-Ternary dekodieren, in `image::RgbaImage` rendern. Standardfarben verwenden (DST hat keine RGB-Werte).
3. **JEF/VP3:** Stich-Koordinaten parsen, in `image::RgbaImage` rendern.
4. Thumbnails im `{metadata_root}/thumbnails/` cachen.

**Methoden:** `generate()`, `get_cached()`, `invalidate()`

**Akzeptanzkriterien:**
- [ ] PES-Thumbnails werden aus eingebettetem Bild generiert
- [ ] DST-Thumbnails werden aus Stich-Koordinaten gerendert
- [ ] Thumbnails werden als PNG im Cache-Verzeichnis gespeichert
- [ ] Erneuter Aufruf nutzt Cache (kein erneutes Rendern)
- [ ] `invalidate()` loescht gecachtes Thumbnail

---

### S6-T4: MetadataPanel-Komponente (Grundversion)

**Beschreibung:** Rechtes Panel mit Vorschau und Datei-Informationen gemaess Proposal §5.5.

**Dateien:**
- `src/components/MetadataPanel.ts`
- `src/styles/components.css` — MetadataPanel-Styles

**Funktionen (Grundversion):**
- Thumbnail-Vorschau (Bild oder Platzhalter)
- Dateiinformationen: Name, Format, Abmessungen, Stichzahl, Farbanzahl
- Farb-Swatches: Hex-Werte und Markennamen (PES) bzw. Platzhalter (DST)
- Read-Only in diesem Sprint (Formular kommt in Sprint 7)

**State-Bindings:** `selectedFileId`

**Akzeptanzkriterien:**
- [ ] Bei Dateiauswahl: Thumbnail und Metadaten angezeigt
- [ ] Farb-Swatches mit korrekten Hex-Werten
- [ ] PES: Farbname + Marke angezeigt
- [ ] DST: Hinweis "Keine Farbinformationen" + Platzhalter-Farben
- [ ] Kein Datei selektiert: Leerer Zustand mit Hinweis

---

## Sprint 7 — Metadaten, Tags & KI-Vorbereitung (Wochen 8–9)

**Ziel:** Vollstaendiges Metadaten-Formular, Tag-System, Toolbar und StatusBar.

**Abhaengigkeiten:** Sprint 4 (File-Commands), Sprint 6 (MetadataPanel Grundversion)

### S7-T1: File update/delete Commands

**Beschreibung:** Schreib- und Loesch-Commands fuer Dateien gemaess Proposal §4.2.1.

**Dateien:**
- `src-tauri/src/commands/files.rs` (Erweiterung)

**Neue Commands:**
- `update_file(file_id, updates: FileUpdate) → EmbroideryFile`
- `delete_file(file_id) → ()`
- `set_file_tags(file_id, tag_names: Vec<String>) → Vec<Tag>`
- `get_thumbnail(file_id) → String` (Base64-encoded)

**Akzeptanzkriterien:**
- [ ] `update_file` aktualisiert name, theme, description, license
- [ ] `delete_file` loescht Datei und alle Relationen (CASCADE)
- [ ] `set_file_tags` erstellt fehlende Tags, setzt Zuordnung
- [ ] `get_thumbnail` gibt Base64-codierten Thumbnail-String zurueck
- [ ] `cargo test` — Update- und Delete-Tests

---

### S7-T2: Settings-Commands

**Beschreibung:** Tauri-Commands fuer Einstellungen gemaess Proposal §4.2.6.

**Dateien:**
- `src-tauri/src/commands/settings.rs`
- `src-tauri/src/commands/mod.rs` — Registrierung

**Commands:**
- `get_setting(key) → String`
- `set_setting(key, value) → ()`
- `get_all_settings() → HashMap<String, String>`
- `get_custom_fields() → Vec<CustomFieldDef>`
- `create_custom_field(name, field_type, options?) → CustomFieldDef`
- `delete_custom_field(field_id) → ()`

**Akzeptanzkriterien:**
- [ ] Alle 6 Commands implementiert
- [ ] `set_setting` aktualisiert `updated_at`
- [ ] `create_custom_field` validiert `field_type` (text, number, date, select)
- [ ] `cargo test` — Settings CRUD-Tests

---

### S7-T3: SettingsService (Frontend)

**Beschreibung:** Frontend-Service fuer Einstellungen.

**Dateien:**
- `src/services/SettingsService.ts`

**Methoden:**
- `get(key): Promise<string>`
- `set(key, value): Promise<void>`
- `getAll(): Promise<Record<string, string>>`
- `getCustomFields(): Promise<CustomFieldDef[]>`
- `createCustomField(name, fieldType, options?): Promise<CustomFieldDef>`
- `deleteCustomField(fieldId): Promise<void>`

**Akzeptanzkriterien:**
- [ ] Alle Methoden implementiert
- [ ] TypeScript kompiliert

---

### S7-T4: MetadataPanel — Formular-Erweiterung

**Beschreibung:** MetadataPanel um editierbare Formularfelder erweitern.

**Dateien:**
- `src/components/MetadataPanel.ts` (Erweiterung)
- `src/styles/components.css` — Formular-Styles

**Felder:**
- Name (Text-Input)
- Thema (Text-Input)
- Beschreibung (Textarea)
- Lizenz (Text-Input)
- Tags (Chip-Eingabe mit Autocomplete)
- Benutzerdefinierte Felder (dynamisch aus DB)

**Akzeptanzkriterien:**
- [ ] Alle Felder editierbar
- [ ] Tag-Eingabe: Chips mit X-Button, Autocomplete bei vorhandenen Tags
- [ ] Benutzerdefinierte Felder werden dynamisch gerendert
- [ ] Formular zeigt aktuelle DB-Werte an

---

### S7-T5: Speichern-Logik

**Beschreibung:** Speichern-Button im MetadataPanel, der Aenderungen an das Backend sendet.

**Dateien:**
- `src/components/MetadataPanel.ts` (Erweiterung)
- `src/services/FileService.ts` (Erweiterung: `updateFile`, `setTags`)

**Logik:**
1. Dirty-State tracken (Formular geaendert?)
2. Speichern-Button: `FileService.updateFile()` + `FileService.setTags()`
3. Erfolg: Toast-Nachricht, State aktualisieren
4. Fehler: Fehlermeldung anzeigen

**Akzeptanzkriterien:**
- [ ] Speichern-Button nur aktiv bei Aenderungen
- [ ] Aenderungen werden in der DB persistiert
- [ ] State wird nach Speichern aktualisiert
- [ ] Fehlermeldung bei Speicher-Fehler

---

### S7-T6: Toolbar-Komponente

**Beschreibung:** Aktions-Toolbar gemaess Proposal §5.5.

**Dateien:**
- `src/components/Toolbar.ts`
- `src/styles/components.css` — Toolbar-Styles

**Aktions-Buttons:**
- Ordner hinzufuegen (oeffnet Datei-Dialog)
- Ordner scannen (startet Verzeichnis-Scan)
- Speichern (Metadaten speichern)
- KI Analyse (startet AI-Analyse — in Sprint 8 funktional)
- Einstellungen (oeffnet SettingsDialog — in Sprint 10 funktional)

**Akzeptanzkriterien:**
- [ ] Toolbar im oberen Bereich (grid-area: toolbar)
- [ ] Buttons mit Icons/Labels
- [ ] Ordner hinzufuegen oeffnet nativen Ordner-Dialog
- [ ] Scan-Button startet Verzeichnis-Scan fuer ausgewaehlten Ordner
- [ ] KI- und Einstellungen-Buttons sind vorhanden (noch ohne volle Funktion)

---

### S7-T7: StatusBar-Komponente

**Beschreibung:** Untere Statusleiste mit Datei-Statistiken gemaess Proposal §5.5.

**Dateien:**
- `src/components/StatusBar.ts`
- `src/styles/components.css` — StatusBar-Styles

**Anzeige:**
- Aktueller Ordner-Name
- Datei-Zaehler: Gesamt, pro Format (z.B. "42 Dateien — 15 PES, 20 DST, 5 JEF, 2 VP3")
- AI-Status (z.B. "12 analysiert")

**State-Bindings:** `files`, `selectedFolderId`

**Akzeptanzkriterien:**
- [ ] StatusBar im unteren Bereich (grid-area: status)
- [ ] Korrekte Datei-Zaehlung nach Format
- [ ] Aktualisiert sich bei Ordner-Wechsel und Datei-Aenderungen

---

## Sprint 8 — KI-Integration (Wochen 10–12)

**Ziel:** AI-Analyse ueber Ollama und OpenAI funktioniert.

**Abhaengigkeiten:** Sprint 6 (ThumbnailGenerator), Sprint 7 (Settings, MetadataPanel)

### S8-T1: AI-Client (Rust)

**Beschreibung:** HTTP-Client fuer Ollama und OpenAI Vision-API gemaess Proposal §4.5.

**Dateien:**
- `src-tauri/src/services/ai_client.rs`
- `src-tauri/src/services/mod.rs`

**Implementierung:**
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
```

**Methoden:**
- `analyze(image_base64, prompt) → AiResponse` — Bild + Prompt an Vision-Modell senden
- `test_connection() → bool` — Verbindungstest

**Akzeptanzkriterien:**
- [ ] Ollama-API: POST `/api/generate` mit Bild und Prompt
- [ ] OpenAI-API: POST `/v1/chat/completions` mit Vision-Payload
- [ ] Timeout konfigurierbar
- [ ] `test_connection` prueft Erreichbarkeit
- [ ] `cargo check` kompiliert

---

### S8-T2: commands/ai.rs

**Beschreibung:** Tauri-Commands fuer KI-Analyse gemaess Proposal §4.2.4.

**Dateien:**
- `src-tauri/src/commands/ai.rs`
- `src-tauri/src/commands/mod.rs` — Registrierung

**Commands:**
- `ai_analyze_file(file_id, app_handle) → AiAnalysisResult` — Emittiert `ai:start`, `ai:complete`, `ai:error`
- `ai_accept_result(result_id) → EmbroideryFile` — Ergebnisse in Metadaten uebernehmen
- `ai_reject_result(result_id) → ()`
- `ai_build_prompt(file_id) → AiPromptPreview` — Prompt-Vorschau generieren
- `ai_test_connection() → AiConnectionStatus`

**Logik fuer `ai_analyze_file`:**
1. Thumbnail laden (Base64)
2. Prompt bauen mit Datei-Metadaten (Abmessungen, Stichzahl, Farben)
3. AI-Client aufrufen
4. Antwort parsen (Name, Thema, Beschreibung, Tags, Farben)
5. Ergebnis in `ai_analysis_results` speichern

**Akzeptanzkriterien:**
- [ ] Alle 5 Commands implementiert und registriert
- [ ] Events werden korrekt emittiert
- [ ] `ai_accept_result` ueberschreibt Metadaten und setzt `ai_analyzed=1`, `ai_confirmed=1`
- [ ] Prompt-Preview zeigt angereicherten Prompt

---

### S8-T3: AiService (Frontend)

**Beschreibung:** Frontend-Service fuer KI-Operationen.

**Dateien:**
- `src/services/AiService.ts`

**Methoden:**
- `analyzeFile(fileId): Promise<AiAnalysisResult>`
- `acceptResult(resultId): Promise<EmbroideryFile>`
- `rejectResult(resultId): Promise<void>`
- `buildPrompt(fileId): Promise<AiPromptPreview>`
- `testConnection(): Promise<AiConnectionStatus>`

**Akzeptanzkriterien:**
- [ ] Alle Methoden implementiert
- [ ] TypeScript kompiliert

---

### S8-T4: AiPreviewDialog

**Beschreibung:** Dialog zur Prompt-Vorschau vor dem Senden gemaess Proposal §5.6.

**Dateien:**
- `src/dialogs/AiPreviewDialog.ts`
- `src/styles/dialogs.css` — Dialog-Styles

**Layout (800 × 600):**
- Links: Prompt-Text (editierbar)
- Rechts: Datei-Vorschau (Thumbnail + Basis-Metadaten)
- Buttons: "Senden", "Abbrechen"

**Akzeptanzkriterien:**
- [ ] Split-View mit Prompt und Vorschau
- [ ] Prompt ist editierbar
- [ ] "Senden" startet die Analyse
- [ ] Dialog schliesst bei "Abbrechen" ohne Aktion

---

### S8-T5: AiResultDialog

**Beschreibung:** Dialog zum Review der KI-Ergebnisse gemaess Proposal §5.6.

**Dateien:**
- `src/dialogs/AiResultDialog.ts`
- `src/styles/dialogs.css`

**Layout (640 × 500):**
- Checkbox-Felder pro KI-Ergebnis (Name, Thema, Beschreibung, Tags)
- Farb-Vergleich: Parser-Farben vs. KI-Farben
- Buttons: "Akzeptieren" (ausgewaehlte), "Alle akzeptieren", "Ablehnen"

**Akzeptanzkriterien:**
- [ ] Jedes KI-Feld einzeln akzeptierbar (Checkbox)
- [ ] Farb-Vergleich visuell dargestellt (Swatches nebeneinander)
- [ ] "Akzeptieren" uebernimmt nur ausgewaehlte Felder
- [ ] "(KI-generiert)"-Label bei uebernommenen Werten

---

### S8-T6: SettingsDialog — KI-Tab

**Beschreibung:** KI-Einstellungen im SettingsDialog.

**Dateien:**
- `src/dialogs/SettingsDialog.ts` (Erstellung, nur KI-Tab in diesem Sprint)
- `src/styles/dialogs.css`

**Felder:**
- Provider: Dropdown (Ollama, OpenAI)
- URL: Text-Input
- API-Key: Passwort-Input (nur bei OpenAI sichtbar)
- Modell: Text-Input
- Temperatur: Slider (0.0–1.0)
- Timeout: Number-Input (ms)
- "Verbindung testen"-Button mit Status-Anzeige

**Akzeptanzkriterien:**
- [ ] KI-Tab mit allen Feldern
- [ ] Provider-Wechsel blendet API-Key-Feld ein/aus
- [ ] Verbindungstest zeigt Erfolg/Fehler
- [ ] Einstellungen werden in der DB gespeichert

---

### S8-T7: AI-Badge in FileList

**Beschreibung:** KI-Analyse-Status als Badge in der FileList.

**Dateien:**
- `src/components/FileList.ts` (Erweiterung)
- `src/styles/components.css`

**Badge-Varianten:**
- Kein Badge: Nicht analysiert
- Gelbes Badge: Analysiert, nicht bestaetigt
- Gruenes Badge: Analysiert und bestaetigt

**Akzeptanzkriterien:**
- [ ] Badge wird in Mini-Card angezeigt
- [ ] 3 Zustaende visuell unterscheidbar
- [ ] Badge aktualisiert sich nach AI-Analyse

---

## Sprint 9 — Batch-Operationen & USB-Export (Wochen 13–14)

**Ziel:** Batch-Rename, -Organize, USB-Export und Batch-KI-Analyse.

**Abhaengigkeiten:** Sprint 8 (KI-Integration), Sprint 7 (File-Commands)

### S9-T1: commands/batch.rs

**Beschreibung:** Batch-Operationen gemaess Proposal §4.2.5.

**Dateien:**
- `src-tauri/src/commands/batch.rs`
- `src-tauri/src/commands/mod.rs` — Registrierung

**Commands:**
- `batch_rename(file_ids, pattern, app_handle) → BatchResult` — Emittiert `batch:progress`, `batch:complete`
- `batch_organize(file_ids, pattern, app_handle) → BatchResult`
- `batch_export_usb(file_ids, target_path) → BatchResult`

**Muster-Ersetzung:**
- `{name}` — Anzeigename
- `{theme}` — Thema
- `{format}` — Dateiformat
- Pattern aus Settings: `rename_pattern`, `organize_pattern`

**Akzeptanzkriterien:**
- [ ] Alle 3 Commands implementiert
- [ ] Muster-Ersetzung mit Platzhaltern funktioniert
- [ ] Fortschritts-Events werden emittiert (pro Datei)
- [ ] `batch_export_usb` kopiert Dateien auf Zielpfad
- [ ] Fehlerhafte Einzeldateien ueberspringen, nicht abbrechen
- [ ] `cargo test` — Rename-Muster-Test, Organize-Logik-Test

---

### S9-T2: BatchService (Frontend)

**Beschreibung:** Frontend-Service fuer Batch-Operationen.

**Dateien:**
- `src/services/BatchService.ts`

**Methoden:**
- `rename(fileIds, pattern): Promise<BatchResult>`
- `organize(fileIds, pattern): Promise<BatchResult>`
- `exportUsb(fileIds, targetPath): Promise<BatchResult>`

**Akzeptanzkriterien:**
- [ ] Alle Methoden implementiert
- [ ] TypeScript kompiliert

---

### S9-T3: BatchDialog

**Beschreibung:** Dialog mit Fortschrittsanzeige gemaess Proposal §5.6.

**Dateien:**
- `src/dialogs/BatchDialog.ts`
- `src/styles/dialogs.css`

**Layout (480 × 400):**
- Fortschrittsbalken (X von N Dateien)
- Log-View (scrollbar): Zeigt jede verarbeitete Datei
- Step-Indikator: Aktueller Schritt
- Abbrechen-Button

**Akzeptanzkriterien:**
- [ ] Fortschrittsbalken zeigt aktuellen Stand
- [ ] Log-View zeigt Dateinamen und Erfolg/Fehler
- [ ] Abbrechen-Button stoppt die Operation
- [ ] Dialog schliesst automatisch bei Erfolg (nach kurzer Verzoegerung)

---

### S9-T4: SettingsDialog — Dateiverwaltung-Tab

**Beschreibung:** Einstellungen fuer Batch-Operationen im SettingsDialog.

**Dateien:**
- `src/dialogs/SettingsDialog.ts` (Erweiterung)

**Felder:**
- Umbennungsmuster: Text-Input mit Platzhalter-Legende (Default: `{name}_{theme}`)
- Organisationsmuster: Text-Input (Default: `{theme}/{name}`)
- Bibliotheks-Stammverzeichnis: Text-Input + Ordner-Dialog
- Metadaten-Verzeichnis: Text-Input

**Akzeptanzkriterien:**
- [ ] Dateiverwaltung-Tab mit allen Feldern
- [ ] Platzhalter-Legende erklaert verfuegbare Variablen
- [ ] Einstellungen werden in der DB gespeichert

---

### S9-T5: Batch-KI-Analyse

**Beschreibung:** Mehrere Dateien sequenziell per KI analysieren.

**Dateien:**
- `src-tauri/src/commands/ai.rs` (Erweiterung)
- `src/services/AiService.ts` (Erweiterung)

**Neuer Command:**
- `ai_analyze_batch(file_ids, app_handle) → Vec<AiAnalysisResult>`

**Logik:**
1. Dateien sequenziell analysieren (nicht parallel — API-Last begrenzen)
2. Pro Datei: Fortschritts-Event emittieren
3. Ergebnisse sammeln und zurueckgeben

**Akzeptanzkriterien:**
- [ ] Batch-Analyse verarbeitet alle Dateien sequenziell
- [ ] Fortschritts-Events pro Datei
- [ ] Fehler bei Einzeldatei ueberspringt, bricht nicht ab
- [ ] Ergebnisse in `ai_analysis_results` gespeichert

---

### S9-T6: Mehrfachauswahl in FileList

**Beschreibung:** FileList um Mehrfachauswahl erweitern fuer Batch-Operationen.

**Dateien:**
- `src/components/FileList.ts` (Erweiterung)
- `src/state/AppState.ts` (Erweiterung: `selectedFileIds: number[]`)

**Logik:**
- Cmd/Ctrl+Klick: Einzelne Datei zur Auswahl hinzufuegen/entfernen
- Shift+Klick: Bereich auswaehlen
- Toolbar reagiert auf Mehrfachauswahl (Batch-Aktionen anzeigen)

**Akzeptanzkriterien:**
- [ ] Cmd/Ctrl+Klick fuer Mehrfachauswahl
- [ ] Shift+Klick fuer Bereichsauswahl
- [ ] Visuelles Feedback: Alle ausgewaehlten Karten hervorgehoben
- [ ] State-Update: `selectedFileIds` Array
- [ ] Toolbar zeigt Batch-Aktionen bei Mehrfachauswahl

---

## Sprint 10 — Polish & Release (Wochen 15–16)

**Ziel:** Produktionsreife, UX-Verbesserungen, Bundle-Build.

**Abhaengigkeiten:** Alle vorherigen Sprints

### S10-T1: SettingsDialog komplett

**Beschreibung:** Alle 5 Tabs des SettingsDialog gemaess Proposal §5.6.

**Dateien:**
- `src/dialogs/SettingsDialog.ts` (Erweiterung)

**Tabs:**
1. **Allgemein** — Bibliotheks-Root, Metadaten-Root (bereits in S9-T4)
2. **Erscheinungsbild** — Theme-Toggle (hell/dunkel), Schriftgroesse
3. **KI** — Provider, URL, API-Key, Modell, Temperatur (bereits in S8-T6)
4. **Dateiverwaltung** — Rename/Organize-Muster (bereits in S9-T4)
5. **Benutzerdefiniert** — Custom Fields erstellen/loeschen

**Akzeptanzkriterien:**
- [ ] Alle 5 Tabs vorhanden und funktional
- [ ] Tab-Navigation mit visueller Hervorhebung
- [ ] Benutzerdefinierte Felder: Erstellen (Name, Typ, Optionen), Loeschen
- [ ] Speichern-Button persistiert alle Aenderungen

---

### S10-T2: Keyboard-Shortcuts

**Beschreibung:** Globale Tastenkuerzel gemaess Proposal §7.

**Dateien:**
- `src/main.ts` (oder eigenes Modul `src/shortcuts.ts`)

**Shortcuts:**
- `Cmd/Ctrl+S` — Metadaten speichern
- `Cmd/Ctrl+F` — Suchfeld fokussieren
- `Cmd/Ctrl+,` — Einstellungen oeffnen
- `Delete/Backspace` — Ausgewaehlte Datei loeschen (mit Bestaetigung)
- `Arrow Up/Down` — Dateiauswahl navigieren
- `Escape` — Dialog schliessen / Auswahl aufheben

**Akzeptanzkriterien:**
- [ ] Alle Shortcuts funktionieren
- [ ] Keine Konflikte mit Browser/WebView-Shortcuts
- [ ] Delete erfordert Bestaetigung

---

### S10-T3: Dateisystem-Watcher

**Beschreibung:** Automatisches Erkennen neuer/geaenderter Dateien gemaess Proposal §7.

**Dateien:**
- `src-tauri/src/services/file_watcher.rs`
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/lib.rs` — Watcher im Setup starten

**Implementierung:**
- `notify`-Crate: Ueberwacht das Bibliotheks-Stammverzeichnis
- Bei neuen .pes/.dst/.jef/.vp3-Dateien: Event emittieren (`fs:new-files`)
- Bei geloeschten Dateien: Event emittieren (`fs:files-removed`)
- Frontend reagiert und aktualisiert Dateien

**Akzeptanzkriterien:**
- [ ] Watcher startet beim App-Start
- [ ] Neue Stickdateien werden erkannt
- [ ] Geloeschte Dateien werden erkannt
- [ ] Frontend aktualisiert sich automatisch

---

### S10-T4: Splitter-Handles

**Beschreibung:** Draggable Splitter zwischen den 3 Panels.

**Dateien:**
- `src/components/Splitter.ts`
- `src/styles/layout.css`

**Logik:**
- Vertikaler Splitter zwischen Sidebar und Center
- Vertikaler Splitter zwischen Center und Right
- Maus-Drag aktualisiert CSS Custom Properties (`--sidebar-width`, `--center-width`)
- Mindestbreiten einhalten

**Akzeptanzkriterien:**
- [ ] Splitter visuell sichtbar (duenner Griff)
- [ ] Panel-Breiten per Drag anpassbar
- [ ] Mindestbreiten werden eingehalten
- [ ] Cursor wechselt bei Hover (`col-resize`)

---

### S10-T5: Virtual Scrolling fuer FileList

**Beschreibung:** Performance-Optimierung fuer grosse Dateilisten (>500 Dateien).

**Dateien:**
- `src/components/FileList.ts` (Erweiterung)

**Implementierung:**
- Nur sichtbare Mini-Cards im DOM rendern
- Scroll-Position tracken
- Bei Scroll: Neue Cards rendern, unsichtbare entfernen
- Feste Card-Hoehe (72px) fuer Berechnung

**Akzeptanzkriterien:**
- [ ] Bei 1000+ Dateien: Fliessender Scroll ohne Ruckeln
- [ ] DOM enthaelt nur ~20–30 Cards (nicht alle)
- [ ] Scrollbar-Position korrekt (Gesamt-Hoehe simuliert)
- [ ] Auswahl funktioniert weiterhin korrekt

---

### S10-T6: Toast-Benachrichtigungen

**Beschreibung:** Benutzer-Feedback fuer Aktionen (Speichern, Fehler, Import, etc.).

**Dateien:**
- `src/components/Toast.ts`
- `src/styles/components.css` — Toast-Styles

**Typen:**
- Erfolg (gruen): "Metadaten gespeichert", "Import abgeschlossen"
- Fehler (rot): "Speichern fehlgeschlagen", "Verbindungsfehler"
- Info (blau): "Scan gestartet", "KI-Analyse laeuft"

**Verhalten:**
- Erscheint oben rechts
- Verschwindet nach 3–5 Sekunden
- Stapelbar (mehrere Toasts uebereinander)

**Akzeptanzkriterien:**
- [ ] 3 Toast-Varianten visuell unterscheidbar
- [ ] Auto-Dismiss nach konfigurierter Zeit
- [ ] Mehrere Toasts stapelbar
- [ ] Animation: Slide-In / Fade-Out

---

### S10-T7: Bundle-Konfiguration

**Beschreibung:** Tauri-Bundle fuer Produktion konfigurieren.

**Dateien:**
- `src-tauri/tauri.conf.json` — Bundle-Settings
- App-Icon vorbereiten

**Konfiguration:**
- Bundle-Identifier: `de.carpeasrael.stichman`
- macOS: `.dmg`-Paket
- Windows: `.msi`-Installer
- Linux: `.deb` / `.AppImage`
- App-Icon fuer alle Plattformen
- Versionsnummer: `2.0.0`

**Akzeptanzkriterien:**
- [ ] `npm run tauri build` erzeugt funktionierendes Bundle
- [ ] Bundle-Groesse < 20 MB (macOS .dmg)
- [ ] App-Icon korrekt angezeigt
- [ ] Kaltstart < 1 Sekunde

---

### S10-T8: Abschluss-QA und Bugfixes

**Beschreibung:** Abschliessende Qualitaetssicherung ueber alle Features.

**Pruef-Checkliste:**

**Ordner-Verwaltung:**
- [ ] Erstellen, Umbenennen, Loeschen
- [ ] Verschachtelte Ordner

**Datei-Import:**
- [ ] Verzeichnis-Scan erkennt alle 4 Formate
- [ ] Duplikate werden erkannt
- [ ] Fortschrittsanzeige

**Parser:**
- [ ] PES: Alle Metadaten korrekt (Dimensionen, Farben, Thumbnail)
- [ ] DST: Header und Stiche korrekt, keine Farb-RGB
- [ ] JEF: Metadaten und Farben
- [ ] VP3: Metadaten und Farben

**Metadaten:**
- [ ] Formular editierbar und speicherbar
- [ ] Tags funktionieren
- [ ] Benutzerdefinierte Felder funktionieren

**KI-Integration:**
- [ ] Ollama-Analyse funktioniert
- [ ] OpenAI-Analyse funktioniert
- [ ] Prompt-Vorschau und Ergebnis-Review
- [ ] Batch-Analyse

**Batch-Operationen:**
- [ ] Rename mit Muster
- [ ] Organize mit Ordnerstruktur
- [ ] USB-Export

**UI/UX:**
- [ ] Theme-Toggle (hell/dunkel)
- [ ] Keyboard-Shortcuts
- [ ] Virtual Scrolling bei grossen Listen
- [ ] Toast-Benachrichtigungen
- [ ] Splitter-Handles
- [ ] Responsive bei verschiedenen Fenstergroessen

**Performance:**
- [ ] Kaltstart < 1 s
- [ ] Scan von 100 Dateien < 5 s
- [ ] Fliessender Scroll bei 1000+ Dateien

**Akzeptanzkriterien:**
- [ ] Alle Punkte der Checkliste bestanden
- [ ] Keine bekannten kritischen Bugs
- [ ] Cross-Platform-Build erfolgreich (macOS + Linux mindestens)

---

## Abhaengigkeitsgraph

```
Sprint 1 (Backend Fundament)
    ├── Sprint 2 (Frontend Fundament)
    │       ├── Sprint 3 (Ordner)
    │       │       └── Sprint 4 (Datei-Import)
    │       │               └── Sprint 7 (Metadaten & Tags)
    │       │                       ├── Sprint 8 (KI)
    │       │                       │       └── Sprint 9 (Batch)
    │       │                       └── Sprint 9 (Batch)
    │       └── Sprint 7 (Metadaten & Tags)
    ├── Sprint 5 (PES & DST Parser)
    │       └── Sprint 6 (JEF, VP3 & Thumbnails)
    │               └── Sprint 7 (Metadaten & Tags)
    └── Sprint 10 (Polish & Release) ← abhaengig von allen
```

**Parallele Arbeit moeglich:**
- Sprint 3 + Sprint 5 koennen parallel bearbeitet werden
- Sprint 4 + Sprint 6 koennen parallel bearbeitet werden (sofern verschiedene Entwickler)

---

## Referenzen

| Dokument | Pfad | Beschreibung |
|----------|------|--------------|
| Technisches Proposal | `release_26.03-a1/init_01.md` | Quelle fuer alle Architektur-Entscheidungen |
| DST-Format-Analyse | `basic/dst_format_analysis.md` | Reverse-Engineered DST-Spezifikation |
| PES-Format-Analyse | `basic/pes_format_analysis.md` | Reverse-Engineered PES-Spezifikation |
| DST-Referenz | `basic/test/extract_dst.py` | Python-Referenzimplementierung DST-Parser |
| PES-Referenz | `basic/test/extract_pes.py` | Python-Referenzimplementierung PES-Parser |
| CLAUDE.md | `CLAUDE.md` | Review-Prozess und Definition of Done |

---

*Ende des Sprint-Plans.*
*Naechster Schritt: Sprint 1, Ticket S1-T1 beginnen.*

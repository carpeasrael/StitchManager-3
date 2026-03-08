# Sprint 3 Analyse: Ordner-Verwaltung

> Release: 26.03-a1 | Sprint: 3 | Datum: 2026-03-08

---

## 1. Problembeschreibung

Die Applikation hat derzeit keine Moeglichkeit, Ordner zu verwalten. Das Datenbankschema (v1-Migration) definiert bereits die `folders`-Tabelle mit hierarchischer Struktur (`parent_id`), aber es existieren weder Backend-Commands noch Frontend-Services oder UI-Komponenten, um Ordner anzulegen, aufzulisten, umzubenennen oder zu loeschen.

Sprint 3 schliesst diese Luecke: Vom Rust-Backend ueber den TypeScript-Service-Layer bis zur Sidebar-Komponente soll die vollstaendige Ordner-CRUD-Funktionalitaet implementiert werden.

---

## 2. Betroffene Komponenten

### Backend (Rust / Tauri)

| Datei | Status | Aenderung |
|---|---|---|
| `src-tauri/src/commands/folders.rs` | **Neu** | 5 Tauri-Commands fuer Ordner-CRUD |
| `src-tauri/src/commands/mod.rs` | Vorhanden (leer) | Re-Export des `folders`-Moduls |
| `src-tauri/src/lib.rs` | Vorhanden | Commands in `invoke_handler` registrieren |
| `src-tauri/capabilities/default.json` | Vorhanden | Verifizieren, dass keine zusaetzlichen Permissions noetig sind |

### Frontend (TypeScript)

| Datei | Status | Aenderung |
|---|---|---|
| `src/services/FolderService.ts` | **Neu** | Service-Klasse mit statischen Methoden, kapselt `invoke()`-Aufrufe |
| `src/components/Sidebar.ts` | **Neu** | UI-Komponente: Ordner-Liste, Auswahl, Neu-Ordner-Button |
| `src/styles/components.css` | **Neu** | CSS fuer Sidebar-Elemente (Ordner-Items, Active-State, Button) |
| `src/main.ts` | Vorhanden | Sidebar instanziieren und in `.app-sidebar` mounten |
| `src/styles.css` | Vorhanden | Import der neuen `components.css` |

### Bestehende Infrastruktur (unveraendert, aber genutzt)

- `src-tauri/src/db/models.rs` — `Folder`-Struct (bereits definiert, Zeilen 15-24)
- `src-tauri/src/error.rs` — `AppError`-Enum mit `Validation`, `NotFound`, `Database` Varianten
- `src/types/index.ts` — `Folder`-Interface (bereits definiert, Zeilen 1-9)
- `src/state/AppState.ts` — `folders` und `selectedFolderId` State-Keys (bereits im `State`-Interface)
- `src/components/Component.ts` — Abstrakte Basisklasse fuer UI-Komponenten

---

## 3. Ursache / Begruendung

Die Ordner-Verwaltung ist eine Kernfunktionalitaet der Applikation. Jede `embroidery_files`-Zeile hat einen Pflicht-Fremdschluessel `folder_id` auf `folders`. Ohne Ordner-CRUD koennen keine Dateien importiert oder organisiert werden. Sprint 3 liefert die Grundlage fuer Sprint 4 (Datei-Import/Scanner) und alle nachfolgenden Features.

Die hierarchische Ordnerstruktur (`parent_id`) ist bereits im Schema angelegt und muss von Anfang an korrekt unterstuetzt werden, damit spaetere Refactorings vermieden werden.

---

## 4. Vorgehensweise (pro Ticket)

### S3-T1: `commands/folders.rs` (Backend)

**Datei:** `src-tauri/src/commands/folders.rs`

1. Erstelle `src-tauri/src/commands/folders.rs` mit 5 Tauri-Commands:

   ```
   #[tauri::command]
   async fn get_folders(state: State<'_, DbState>) -> Result<Vec<Folder>, AppError>
   ```
   - Lock den Mutex: `state.0.lock().unwrap()`
   - Query: `SELECT id, name, path, parent_id, sort_order, created_at, updated_at FROM folders ORDER BY sort_order, name`
   - Map Rows auf `Folder`-Struct via `rusqlite::Row`

   ```
   #[tauri::command]
   async fn create_folder(name: String, path: String, parent_id: Option<i64>, state: State<'_, DbState>) -> Result<Folder, AppError>
   ```
   - Validierung: `name.trim()` nicht leer -> sonst `AppError::Validation`
   - Validierung: `std::path::Path::new(&path).exists()` -> sonst `AppError::Validation`
   - Optional: Wenn `parent_id` angegeben, pruefen ob Elternordner existiert -> sonst `AppError::NotFound`
   - INSERT in `folders`-Tabelle
   - Eingefuegten Ordner per `last_insert_rowid()` zuruecklesen und zurueckgeben

   ```
   #[tauri::command]
   async fn update_folder(folder_id: i64, name: Option<String>, state: State<'_, DbState>) -> Result<Folder, AppError>
   ```
   - Pruefen ob Ordner existiert -> sonst `AppError::NotFound`
   - Wenn `name` angegeben: Validierung nicht leer, dann UPDATE
   - `updated_at` auf `datetime('now')` setzen
   - Aktualisierten Ordner zurueckgeben

   ```
   #[tauri::command]
   async fn delete_folder(folder_id: i64, state: State<'_, DbState>) -> Result<(), AppError>
   ```
   - Pruefen ob Ordner existiert -> sonst `AppError::NotFound`
   - DELETE — Kaskade loescht automatisch zugehoerige `embroidery_files` (DB-Schema: `ON DELETE CASCADE`)

   ```
   #[tauri::command]
   async fn get_folder_file_count(folder_id: i64, state: State<'_, DbState>) -> Result<i64, AppError>
   ```
   - Query: `SELECT COUNT(*) FROM embroidery_files WHERE folder_id = ?1`

2. **Hilfsfunktion:** Private `query_folder_by_id(conn, id) -> Result<Folder, AppError>` um DRY zu halten (wird von create, update, delete benoetigt).

3. **`src-tauri/src/commands/mod.rs`** aktualisieren:
   ```rust
   pub mod folders;
   pub use folders::*;
   ```

4. **`src-tauri/src/lib.rs`** aktualisieren — `.invoke_handler()` in der Builder-Chain ergaenzen:
   ```rust
   builder = builder.invoke_handler(tauri::generate_handler![
       commands::get_folders,
       commands::create_folder,
       commands::update_folder,
       commands::delete_folder,
       commands::get_folder_file_count,
   ]);
   ```
   Wichtig: `.invoke_handler()` muss VOR `.run()` aufgerufen werden. In `lib.rs` zwischen dem `#[cfg(debug_assertions)]`-Block und dem `builder.run(...)` einfuegen.

5. **Tests** in `commands/folders.rs` schreiben:
   - `test_create_folder_success`
   - `test_create_folder_empty_name_fails`
   - `test_get_folders_empty`
   - `test_delete_folder_cascades`
   - `test_update_folder_name`
   - `test_get_folder_file_count`

   Tests nutzen `db::migrations::init_database_in_memory()` direkt (ohne Tauri-Runtime), indem die Kernlogik in testbare Hilfsfunktionen extrahiert wird, die `&Connection` statt `State<DbState>` nehmen.

### S3-T2: `FolderService.ts` (Frontend Service)

**Datei:** `src/services/FolderService.ts`

1. Erstelle die Datei mit einer Klasse `FolderService` mit statischen async-Methoden:

   ```typescript
   import { invoke } from '@tauri-apps/api/core';
   import type { Folder } from '../types/index';

   export class FolderService {
     static async getAll(): Promise<Folder[]>
     static async create(name: string, path: string, parentId?: number): Promise<Folder>
     static async update(folderId: number, name: string): Promise<Folder>
     static async remove(folderId: number): Promise<void>
     static async getFileCount(folderId: number): Promise<number>
   }
   ```

2. Jede Methode ruft `invoke()` mit snake_case Command-Name auf. Wichtig: Tauri v2 erwartet camelCase-Keys im args-Objekt und wandelt intern zu snake_case um. Beispiel:
   ```typescript
   static async create(name: string, path: string, parentId?: number): Promise<Folder> {
     return invoke('create_folder', {
       name,
       path,
       parentId: parentId ?? null,
     });
   }
   ```

3. Achtung bei `remove()`: Der Methodenname weicht vom Command-Namen ab (`remove` statt `delete`), da `delete` ein reserviertes Wort in striktem JavaScript ist.

### S3-T3: Sidebar-Komponente (Frontend UI)

**Dateien:** `src/components/Sidebar.ts`, `src/styles/components.css`

1. **`src/components/Sidebar.ts`** — Extends `Component`:

   ```typescript
   export class Sidebar extends Component {
     constructor(container: HTMLElement)
     render(): void
     private renderFolderList(folders: Folder[]): void
     private handleFolderClick(folderId: number): void
     private handleCreateFolder(): void
     private loadFolders(): Promise<void>
   }
   ```

   - Im Konstruktor: `appState.on('folders', ...)` und `appState.on('selectedFolderId', ...)` subscriben (via `this.subscribe()`).
   - `render()`: Grundstruktur mit Header ("Ordner"), "+ Neuer Ordner"-Button, und Ordner-Liste aufbauen.
   - `loadFolders()`: Ruft `FolderService.getAll()` auf, fuer jeden Ordner zusaetzlich `FolderService.getFileCount()`, dann `appState.set('folders', folders)`.
   - `handleFolderClick()`: Setzt `appState.set('selectedFolderId', id)`.
   - `handleCreateFolder()`: Zeigt einen einfachen Dialog (oder Inline-Input) fuer Name und Pfad, ruft `FolderService.create()` auf, dann `loadFolders()`.
   - Aktiver Ordner wird via CSS-Klasse `.folder-item--active` hervorgehoben.
   - Jedes Ordner-Item zeigt: Name und Datei-Anzahl in Klammern.

2. **`src/styles/components.css`** — Sidebar-spezifische Styles:

   - `.sidebar-header` — Flex-Layout mit Titel und Button
   - `.folder-list` — Liste ohne default Padding
   - `.folder-item` — Padding, Cursor Pointer, Hover-Effekt, Border-Radius
   - `.folder-item--active` — `background: var(--color-accent-10)`, `color: var(--color-accent)`
   - `.folder-item__count` — `color: var(--color-muted)`, kleiner Font
   - `.btn-create-folder` — Kompakter Button mit Accent-Farbe

   Alle Styles nutzen die bestehenden CSS-Variablen aus `aurora.css`.

3. **`src/main.ts`** aktualisieren:
   - Sidebar importieren und instanziieren:
     ```typescript
     const sidebarEl = document.querySelector('.app-sidebar');
     if (sidebarEl) {
       const sidebar = new Sidebar(sidebarEl as HTMLElement);
       sidebar.render();
     }
     ```
   - `loadFolders()` initial aufrufen.

4. **`src/styles.css`** — Import ergaenzen:
   ```css
   @import './styles/components.css';
   ```

### S3-T4: Tauri-Permissions verifizieren

**Datei:** `src-tauri/capabilities/default.json`

1. In Tauri v2 sind Custom-Commands (registriert via `invoke_handler`) automatisch erlaubt fuer Fenster, die in der Capability gelistet sind. Die bestehende Konfiguration mit `"core:default"` und `"windows": ["main"]` ist ausreichend.

2. **Verifizierung:** Nach Implementierung von S3-T1 bis S3-T3 testen, dass alle 5 Commands vom Frontend erreichbar sind. Falls ein Permission-Fehler auftritt, muss die `default.json` analysiert werden — aber nach aktuellem Wissensstand ist keine Aenderung noetig.

3. **Keine Aenderung** an `default.json` erforderlich.

---

## 5. Implementierungsreihenfolge

| Schritt | Ticket | Begruendung |
|---|---|---|
| 1 | **S3-T1** | Backend-Commands zuerst — Frontend haengt davon ab |
| 2 | **S3-T4** | Permissions verifizieren (schneller Check, blockiert T2/T3 bei Fehler) |
| 3 | **S3-T2** | Service-Layer braucht funktionierende Commands |
| 4 | **S3-T3** | UI-Komponente braucht Service-Layer |

S3-T4 kann parallel zu S3-T1 oder direkt danach bearbeitet werden, da es nur eine Verifikation ist.

---

## 6. Technische Hinweise

### Mutex-Handling im Backend

`DbState(Mutex<Connection>)` wird per `state.0.lock().unwrap()` gelockt. Da Tauri-Commands async sind, muss der Lock-Guard nicht ueber `.await`-Points gehalten werden. Best Practice: Lock akquirieren, DB-Operation ausfuehren, Guard droppen — alles synchron innerhalb des async-Blocks. Die Signatur nutzt `tauri::State<'_, DbState>`.

### Serde-Konvertierung snake_case vs. camelCase

Die Rust-Structs nutzen snake_case Feldnamen (`parent_id`, `sort_order`, `created_at`). Serde serialisiert diese per Default ebenfalls als snake_case in JSON. Das TypeScript-Interface `Folder` nutzt camelCase (`parentId`, `sortOrder`, `createdAt`).

Loesung: Entweder `#[serde(rename_all = "camelCase")]` auf dem Rust-Struct, oder die TypeScript-Seite mappt die Keys. Empfehlung: `#[serde(rename_all = "camelCase")]` auf `Folder` in `models.rs` hinzufuegen — das ist konsistent mit der Tauri-Konvention und vermeidet manuelles Mapping.

**Wichtig:** Diese Aenderung betrifft nur die JSON-Serialisierung, nicht die rusqlite-Row-Abfragen (dort werden Spalten per Index oder exaktem DB-Spaltennamen gelesen).

### Ordner-Erstellung: Pfad-Validierung

`create_folder` validiert, dass der Pfad existiert (`Path::new(&path).exists()`). Dies prueft das Dateisystem — der Ordner muss tatsaechlich auf der Festplatte vorhanden sein. Die App verwaltet Referenzen auf echte Ordner, erstellt sie nicht selbst.

### Cascading Delete

Die DB-Tabelle `embroidery_files` hat `ON DELETE CASCADE` auf `folder_id`. Wenn ein Ordner geloescht wird, werden automatisch alle zugehoerigen Dateien entfernt. Der `delete_folder`-Command muss dies nicht manuell tun — es reicht ein `DELETE FROM folders WHERE id = ?1`. Der bestehende Test `test_cascade_delete_folder_removes_files` in `migrations.rs` bestaetigt dieses Verhalten bereits.

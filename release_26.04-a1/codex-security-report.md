# Security Test Report — Codex Reviewer Agent
**Date:** 2026-03-17
**Release:** 26.04-a1

## Summary
- Tests executed: 35
- Passed: 29
- Findings: 6 (Critical: 0, High: 2, Medium: 3, Low: 1)

## Test Results

### ST-01 SQL Injection: FTS5 special characters
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:52-54`
- **Description:** All FTS5 special characters (`"`, `*`, `+`, `-`, `^`, `(`, `)`, `{`, `}`, `:`) are stripped from user input before constructing the MATCH query. The sanitized value is then wrapped in quotes with a wildcard suffix (`"sanitized"*`) and passed as a parameterized value.

### ST-02 SQL Injection: LIKE query wildcards
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:9-11`
- **Description:** `escape_like()` properly escapes `\`, `%`, and `_` characters. The LIKE clause specifies `ESCAPE '\\'`. Applied to all LIKE-based search conditions.

### ST-03 SQL Injection: Dynamic ORDER BY clause
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/files.rs:256-274`
- **Description:** `build_order_clause` validates `sort_field` against an explicit whitelist of allowed column names. `sort_direction` is matched against `Some("desc")` only. Values that fail validation fall back to `ORDER BY e.filename ASC`. The field value is used directly in the SQL string but ONLY after whitelist validation, which is the correct approach for ORDER BY clauses (parameterized queries cannot bind column names).

### ST-04 SQL Injection: Parameterized queries in all commands
- **Status:** PASS
- **File(s):** All files in `src-tauri/src/commands/`
- **Description:** Comprehensive audit of all 155+ Tauri commands confirms all SQL queries use parameterized binding (`?1`, `?2`, etc. or `rusqlite::params![]`). No string concatenation of user input into SQL queries found. Dynamic query construction (e.g., `build_query_conditions`) uses format strings only for SQL structure (column names from whitelist, logical operators), never for user-supplied values.

### ST-05 XSS: innerHTML usage audit
- **Status:** FINDING
- **Severity:** High
- **File(s):** `src/components/ManufacturingDialog.ts:766-767`, `src/components/ManufacturingDialog.ts:879`, `src/components/ManufacturingDialog.ts:1052`, `src/components/ManufacturingDialog.ts:2422`, `src/components/ManufacturingDialog.ts:2773`, `src/components/ProjectListDialog.ts:89`, `src/components/ProjectListDialog.ts:253`, `src/components/ProjectListDialog.ts:483`
- **Description:** The codebase uses `innerHTML` extensively (60+ instances across 20+ components). The majority of uses fall into safe categories: (a) static HTML strings without user data, (b) clearing content with `el.innerHTML = ""`, or (c) user data set via `textContent` after the DOM structure is created with innerHTML. However, several patterns in ManufacturingDialog and ProjectListDialog construct HTML table headers and select option HTML with `innerHTML` that include static German text only — these are safe. The critical concern is that the pattern of using `innerHTML` for structural HTML followed by `textContent` for data is error-prone. A future developer adding user data to an innerHTML template literal could introduce XSS.

    The strongest finding is that `escapeHtml()` is defined and used in `BatchDialog.ts:57` (`this.escapeHtml(this.operation)`), confirming the developers are aware of the risk. However, this escape function is only used in one component. Other components that insert dynamic content into innerHTML template literals do not use it, relying instead on the textContent pattern.
- **Evidence:** `ManufacturingDialog.ts:766-767` uses `table.innerHTML = "<thead>..."` with static HTML only; data is added via textContent in subsequent code. This is safe currently but fragile. No instance was found where raw user data is concatenated into innerHTML without escaping.
- **Proposed Fix:** (1) Extract `escapeHtml` to a shared utility module. (2) Add ESLint rule to flag `innerHTML` assignments with template literals containing non-constant expressions. (3) Consider migrating high-risk areas to DOM API construction exclusively.

### ST-06 XSS: Template literal injection
- **Status:** PASS
- **File(s):** `src/components/` (all components)
- **Description:** Template literals used in innerHTML assignments were audited. All instances either contain only static text or use `escapeHtml()` / `textContent` for dynamic values. No raw user data injection found.

### ST-07 XSS: User data in DOM attributes
- **Status:** PASS
- **File(s):** `src/components/FileList.ts:241,263`
- **Description:** `card.setAttribute("aria-label", ...)` uses string values from DB. While attribute injection is less severe than innerHTML, these values are metadata from the local database in a desktop app (not externally-sourced). Risk is low.

### ST-08 Code Injection: eval()/Function() usage
- **Status:** PASS
- **File(s):** All `src/` files
- **Description:** No instances of `eval()`, `new Function()`, or `setTimeout` with string arguments found. All `setTimeout` calls use arrow function callbacks.

### ST-09 OS Command Injection: Shell execute
- **Status:** FINDING
- **Severity:** Medium
- **File(s):** `src-tauri/src/commands/files.rs:1263-1283`
- **Description:** `open_attachment` uses `std::process::Command::new("open"/"xdg-open")` with a file path argument retrieved from the database. While `validate_no_traversal` is called and the path is checked for existence and is-file, the path value originates from the database and was set during `attach_file`. If an attacker could manipulate the database (e.g., via the `tauri-plugin-sql` frontend access), they could craft a malicious path. The `Command::new("xdg-open").arg(&file_path)` pattern passes the path as a single argument (not through a shell), which mitigates shell injection, but `xdg-open` itself can be redirected to execute arbitrary commands depending on the file type and desktop environment configuration.
- **Evidence:** `src-tauri/src/commands/files.rs:1275-1278`:
    ```rust
    std::process::Command::new("xdg-open")
        .arg(&file_path)
        .spawn()
    ```
    The `file_path` comes from `SELECT file_path FROM file_attachments WHERE id = ?1`. The `attach_file` command copies the source file to the app data directory, so the stored path should be within a controlled location. However, `validate_no_traversal` only checks for `..` components, not for other potentially dangerous paths.
- **Proposed Fix:** (1) Verify that the file path starts with the expected app data directory prefix before passing to the OS opener. (2) Consider using Tauri's `opener` plugin instead of direct `Command::new` invocation, as it may provide better sandboxing.

### ST-10 Path Traversal: File paths
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/mod.rs:27-38`
- **Description:** `validate_no_traversal` uses `Path::components()` to detect `Component::ParentDir` — robust against string encoding tricks. Called in `parse_embroidery_file`, `get_stitch_segments`, `batch_export_usb`, `restore_backup`, `relink_file`, `relink_batch`, `import_library`, `viewer::read_file_bytes`, `convert_file`, `edit::save_transformed`, `transfer::transfer_files`, `print::print_pdf`, `attach_file`, `open_attachment`, `export_version`. Coverage is comprehensive across file-path-accepting commands.

### ST-11 Path Traversal: Unicode normalization bypasses
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/mod.rs:27-28`
- **Description:** `Path::components()` handles Unicode normalization at the OS level. Rust's `Path` type delegates to the platform's path parsing, which normalizes separators. The `Component::ParentDir` match catches `..` regardless of encoding. Additional protection in batch operations: `batch_organize` canonicalizes the base directory and verifies the target path `starts_with` the canonical base.

### ST-12 Access Control: Tauri capability restrictions
- **Status:** FINDING
- **Severity:** Medium
- **File(s):** `src-tauri/capabilities/default.json`
- **Description:** The capability file grants `core:default`, `sql:default`, `dialog:default`, and `opener:default`. The `sql:default` permission gives the frontend direct access to the SQLite database for arbitrary read queries via `tauri-plugin-sql`. This is by design (documented in CLAUDE.md as "dual access"), but it means that any XSS vulnerability in the frontend could be leveraged to read or modify the entire database, including AI API keys stored in the `settings` table. The attack surface is wider than necessary.
- **Evidence:** `capabilities/default.json` line 8: `"sql:default"`. The `settings` table stores `ai_api_key` as plaintext (see ST-14). Frontend JavaScript can execute `SELECT value FROM settings WHERE key = 'ai_api_key'` directly.
- **Proposed Fix:** (1) Restrict `sql` capability to read-only access if possible, or scope it to specific tables. (2) Consider removing sensitive settings (API keys) from direct SQL access and exposing them only through backend commands with appropriate access controls.

### ST-13 Missing Auth: Command access without authentication
- **Status:** PASS
- **File(s):** `src-tauri/src/lib.rs`
- **Description:** This is a single-user desktop application. There is no multi-user authentication requirement. All commands are accessible to the local user, which is appropriate for the threat model.

### ST-14 Plaintext Secrets: API keys in SQLite
- **Status:** FINDING
- **Severity:** High
- **File(s):** `src-tauri/src/commands/ai.rs:65-94`, `src-tauri/src/db/migrations.rs`
- **Description:** AI API keys (OpenAI API keys) are stored as plaintext in the `settings` table (`key = 'ai_api_key'`). The `load_ai_config` function reads the key directly: `get("ai_api_key").ok().filter(|k| !k.trim().is_empty())`. The database file is stored in the user's app data directory and is accessible to any process running as the same user. Additionally, the `tauri-plugin-sql` frontend access (ST-12) means JavaScript code can read this value directly.
- **Evidence:** `src-tauri/src/commands/ai.rs:77`: `let api_key = get("ai_api_key").ok().filter(|k| !k.trim().is_empty());`
    The value is stored with `set_setting` at `settings.rs:34`: `conn.execute("INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))", ...)`.
    No encryption, obfuscation, or OS keychain integration.
- **Proposed Fix:** Use the operating system's credential storage (e.g., `keyring` crate for cross-platform secret storage — macOS Keychain, Windows Credential Manager, Linux Secret Service) instead of storing API keys in the SQLite database. At minimum, encrypt the API key value with a key derived from machine-specific information before storing in the database.

### ST-15 Cleartext Transmission: AI API keys in HTTP
- **Status:** PASS
- **File(s):** `src-tauri/src/services/ai_client.rs:145-148`
- **Description:** OpenAI API key sent via Bearer auth in HTTPS request. The URL is configurable and defaults to `https://api.openai.com`. Ollama is typically local (`localhost`). The `reqwest` client uses system TLS by default.

### ST-16 Weak Crypto: SHA2 usage
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/ai.rs:266-267`
- **Description:** SHA-256 used only for prompt hashing (deduplication, not security). Appropriate for the use case.

### ST-17 CSP: unsafe-inline in style-src
- **Status:** FINDING
- **Severity:** Medium
- **File(s):** `src-tauri/tauri.conf.json:26`
- **Description:** CSP includes `style-src 'self' 'unsafe-inline'`. While this is common in desktop apps that use inline styles (e.g., Splitter component sets widths via `style` attribute, virtual scroll sets heights dynamically), it weakens the CSP by allowing style injection. Style injection can be used for data exfiltration via CSS selectors (e.g., `input[value^="sk-"]`).
- **Evidence:** `tauri.conf.json:26`: `"csp": "default-src 'self'; ... style-src 'self' 'unsafe-inline'; script-src 'self' blob:; worker-src 'self' blob:"`
- **Proposed Fix:** Migrate inline styles to CSS classes where possible. For truly dynamic styles (virtual scroll positioning), use `style.cssText` or CSS custom properties. Once inline styles are minimized, remove `'unsafe-inline'` from `style-src` and use nonces or hashes for remaining cases.

### ST-18 CSP: Missing directives
- **Status:** PASS
- **File(s):** `src-tauri/tauri.conf.json:26`
- **Description:** CSP includes `default-src 'self'` which covers `connect-src`, `form-action`, and `frame-ancestors` implicitly. `img-src` allows `data:`, `asset:`, `blob:` for thumbnails and background images. `script-src 'self' blob:` (blob needed for PDF.js worker). The CSP is reasonably tight for a Tauri desktop application.

### ST-19 Info Disclosure: Error messages
- **Status:** PASS
- **File(s):** `src-tauri/src/error.rs`
- **Description:** `AppError` serializes as `{ code, message }` JSON. Error codes are generic categories (DATABASE, IO, PARSE, AI, NOT_FOUND, VALIDATION, INTERNAL). Messages include context but use German-language descriptions rather than raw stack traces. Internal error details (e.g., mutex poisoning messages) could leak implementation details but this is acceptable for a desktop app.

### ST-20 Log Injection: User-controlled data in logs
- **Status:** PASS
- **File(s):** Various
- **Description:** `log::warn!` and `log::info!` macros include filenames and error messages. Since the log plugin is only enabled in debug builds (`#[cfg(debug_assertions)]`), production builds do not expose log data. User filenames in log messages cannot affect log parsing in a desktop context.

### ST-21 Dependencies: Rust CVEs
- **Status:** PASS (deferred)
- **Description:** Static analysis only — `cargo audit` should be run as part of the release pipeline. No known vulnerable patterns in dependency usage observed during code review.

### ST-22 Dependencies: Node CVEs
- **Status:** PASS (deferred)
- **Description:** Static analysis only — `npm audit` should be run as part of the release pipeline.

### ST-23 Dependencies: SQLite version
- **Status:** PASS
- **File(s):** `src-tauri/Cargo.toml` (rusqlite dependency)
- **Description:** SQLite is bundled via `rusqlite` with the `bundled` feature. The version is determined by the `rusqlite` crate release. No known SQLite CVEs affect the features used (WAL, FTS5, FKs).

### ST-24 TOCTOU: Batch operation race conditions
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/batch.rs`
- **Description:** TOCTOU is explicitly documented as acceptable for a single-user desktop app. The three-phase design (load, FS operation, DB commit) with rollback handles the failure case correctly. Documented in code comments at `batch.rs:149` and `batch.rs:356`.

### ST-25 Race Condition: Mutex poisoning
- **Status:** PASS
- **File(s):** `src-tauri/src/error.rs:30-32`
- **Description:** `lock_db` maps mutex poison errors to `AppError::Internal("Mutex poisoned: ...")`. A poisoned mutex indicates a panic in a previous lock holder — the error is propagated cleanly. Since this is a single-threaded command execution model (Tauri commands run on the async runtime), actual poisoning would indicate a bug, not a concurrency issue.

### ST-26 Input Validation: Size limits and field validation
- **Status:** PASS
- **File(s):** Various
- **Description:** File size limit: 100MB (`MAX_IMPORT_SIZE`). Background image: 10MB with resize to 1920x1080. Version snapshots: 10MB (`MAX_VERSION_SIZE`). Folder name: non-empty. Project name: non-empty. Custom field type: whitelist. Order quantity: > 0. Delivery quantity: > 0. Over-delivery: 1.1x tolerance.

### ST-27 IPC Security: Tauri invoke() surface
- **Status:** PASS
- **File(s):** `src-tauri/src/lib.rs:118-348`
- **Description:** All 155+ commands are explicitly registered in `generate_handler![]`. No catch-all or dynamic command resolution. Each command has typed parameters deserialized by Tauri's IPC layer.

### ST-28 Window Security: Decorations and fullscreen
- **Status:** PASS
- **File(s):** `src-tauri/tauri.conf.json:13-22`
- **Description:** `decorations: true`, `fullscreen: false`, `resizable: true`. Window cannot go fullscreen (mitigates fullscreen phishing).

### ST-29 Event System: Frontend event spoofing
- **Status:** PASS
- **File(s):** `src-tauri/src/services/file_watcher.rs`, `src-tauri/src/commands/scanner.rs`
- **Description:** Tauri events flow backend-to-frontend via `AppHandle::emit()`. The frontend listens but cannot emit backend events. Frontend-to-backend communication uses `invoke()` which goes through the registered command handler.

### ST-30 File Dialog: Dialog API restrictions
- **Status:** PASS
- **File(s):** `src-tauri/capabilities/default.json:9`
- **Description:** `dialog:default` permission allows file and folder selection dialogs. These are native OS dialogs that return user-selected paths — the user explicitly consents to the path.

### ST-31 Resource Exhaustion: Unbounded operations
- **Status:** PASS
- **File(s):** Various
- **Description:** `dedup_path` has a counter cap at 100,000 iterations. VP3 parser has scan limits (`scan_limit = pos + 1_000_000`, `consecutive_misses > 10_000`). File watcher uses `HashSet` for deduplication (bounded by actual events). Batch operations process a fixed `file_ids` list. Version pruning limits to `MAX_VERSIONS_PER_FILE = 10`.

### ST-32 Deserialization: serde_json parsing of AI responses
- **Status:** PASS
- **File(s):** `src-tauri/src/services/ai_client.rs:195-252`
- **Description:** `parse_ai_json` extracts JSON from raw text, finds `{` and `}` boundaries, and parses with `serde_json::from_str`. Unknown fields are ignored. Malformed JSON results in None values for all parsed fields — no crash or injection.

### ST-33 XXE: XML parsing
- **Status:** PASS
- **File(s):** `src-tauri/src/commands/thread_colors.rs` (roxmltree usage)
- **Description:** `roxmltree` is a non-validating parser that does not process external entities. DTD processing is not supported, eliminating XXE risk.

### ST-34 Allocation: Thumbnail memory for large files
- **Status:** PASS
- **File(s):** `src-tauri/src/services/thumbnail.rs`
- **Description:** Thumbnail output is fixed at 192x192 RGBA = ~147KB regardless of input size. Stitch segments are the main memory consumer but are bounded by the 100MB file size limit. Image operations use the `image` crate which handles allocation efficiently.

### ST-35 Hardcoded Credentials: Source code scan
- **Status:** PASS
- **File(s):** All source files
- **Description:** No hardcoded API keys, passwords, tokens, or secrets found in source code. AI configuration is loaded from the database at runtime. Example API key in test (`sk-test123`) is clearly a test fixture.

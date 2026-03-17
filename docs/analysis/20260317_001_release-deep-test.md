# Analysis: Release Deep Test — Functional, Performance & Security Audit

**Date:** 2026-03-17
**Release:** 26.04-a1
**Scope:** Full application audit — functional correctness, performance, security (OWASP/CWE)

---

## 1. Problem Description

StitchManager requires a comprehensive pre-release quality audit covering:

1. **Functional testing** — all documented features against CLAUDE.md requirements
2. **Performance testing** — virtual scrolling, DB queries, batch operations, memory management
3. **Security testing** — OWASP Top 10, CWE/SANS Top 25, Tauri-specific attack vectors

Each test category will be executed by **two independent agents** (Codex + Claude). All findings must be **cross-validated** by the second agent before being documented as GitHub issues.

---

## 2. Affected Components

### Frontend (47 TypeScript files)
- 23 components (Component base class, Sidebar, FileList, MetadataPanel, Toolbar, SearchBar, FilterChips, StatusBar, Splitter, Toast, BatchDialog, AiPreviewDialog, AiResultDialog, SettingsDialog, ProjectListDialog, ManufacturingDialog, EditDialog, DocumentViewer, ImageViewerDialog, PrintPreviewDialog, TagInput, Dashboard)
- 14 services (FileService, FolderService, AiService, BatchService, ScannerService, SettingsService, ProjectService, PrintService, ViewerService, ThreadColorService, BackupService, ManufacturingService, ProcurementService, ReportService)
- State management (AppState, EventBus)
- Styles (aurora.css, layout.css, components.css)

### Backend (155+ Tauri commands)
- Commands: files, folders, scanner, batch, ai, settings, backup, projects, manufacturing, procurement, reports, print, transfer, viewer, audit
- Database: 39+ tables, FTS5, 21 migrations
- Parsers: PES, DST, JEF, VP3, PDF, Image
- Services: ai_client, file_watcher, thumbnail, usb_monitor

### Configuration
- tauri.conf.json (CSP, window config)
- capabilities/default.json (permissions)
- Cargo.toml (Rust dependencies)
- package.json (Node dependencies)

---

## 3. Test Plan

### 3.1 Functional Tests (FT)

| ID | Category | Test | Expected | Reference |
|----|----------|------|----------|-----------|
| FT-01 | Folders | CRUD operations: create, read, update, delete folder | All operations succeed with proper validation | folders.rs |
| FT-02 | Folders | Empty name rejection | Validation error returned | folders.rs:34 |
| FT-03 | Folders | Non-existent path rejection | Validation error returned | folders.rs:38 |
| FT-04 | Folders | Cascading delete (files in folder) | All child records removed | migrations.rs FK constraints |
| FT-05 | Files | File import via scan_directory | Files parsed, metadata extracted, DB populated | scanner.rs |
| FT-06 | Files | Multi-format support (PES, DST, JEF, VP3) | All four formats parsed correctly | parsers/ |
| FT-07 | Files | PDF and image file support | PDF/image files imported with metadata | parsers/pdf.rs, image_parser.rs |
| FT-08 | Files | Oversized file rejection (>100MB) | Skipped with warning | scanner.rs MAX_IMPORT_SIZE |
| FT-09 | Files | Symlink loop prevention | No infinite traversal | walkdir follow_links(false) |
| FT-10 | Files | File metadata update | All fields persisted correctly | files.rs update_file |
| FT-11 | Files | File deletion (soft delete) | deleted_at set, not returned in queries | files.rs delete_file |
| FT-12 | Files | Trash: restore, purge, auto-purge | Recovery and permanent deletion work | files.rs |
| FT-13 | Files | Favorite toggle | is_favorite toggled, persisted | files.rs toggle_favorite |
| FT-14 | Search | Full-text search (FTS5) | Results match name, description, theme, etc. | files.rs FTS5 query |
| FT-15 | Search | Advanced search (tags, colors, formats, ranges) | All filter combinations produce correct results | files.rs build_query_conditions |
| FT-16 | Search | FTS5 special character sanitization | No query injection possible | files.rs sanitized chars |
| FT-17 | Tags | CRUD tags on files | Tags created, associated, removed correctly | files.rs set_file_tags |
| FT-18 | Tags | Tag autocomplete | All existing tags returned | files.rs get_all_tags |
| FT-19 | Thumbnails | Synthetic thumbnail generation | PNG created for PES/DST/JEF/VP3 | thumbnail.rs |
| FT-20 | Thumbnails | Embedded thumbnail extraction (PES) | PEC thumbnail extracted | pes.rs |
| FT-21 | Thumbnails | Thumbnail caching | Cache hit on repeat requests | thumbnail.rs get_cached |
| FT-22 | Batch | Batch rename with pattern | Files renamed using {name}/{theme}/{format} | batch.rs batch_rename |
| FT-23 | Batch | Batch organize | Files moved to pattern-based directories | batch.rs batch_organize |
| FT-24 | Batch | USB export | Files copied to USB device path | batch.rs batch_export_usb |
| FT-25 | Batch | Three-phase operation (load→FS→commit) | Atomic operation with rollback on failure | batch.rs |
| FT-26 | AI | Prompt building | Correct prompt constructed from file metadata | ai.rs ai_build_prompt |
| FT-27 | AI | Ollama analysis | File analyzed via local LLM | ai_client.rs Ollama |
| FT-28 | AI | OpenAI analysis | File analyzed via OpenAI API | ai_client.rs OpenAI |
| FT-29 | AI | Accept/reject results | Per-field accept/reject persists correctly | ai.rs ai_accept/reject |
| FT-30 | AI | Batch analysis | Multiple files analyzed sequentially | ai.rs ai_analyze_batch |
| FT-31 | AI | Connection test | Provider connectivity verified | ai.rs ai_test_connection |
| FT-32 | Settings | Key-value CRUD | Settings persisted and retrieved | settings.rs |
| FT-33 | Settings | Custom field definitions | Create, list, delete custom fields | settings.rs |
| FT-34 | Settings | Theme mode (hell/dunkel) | Theme applied correctly | SettingsDialog.ts |
| FT-35 | Settings | Background image | Upload, display, remove background | settings.rs |
| FT-36 | Backup | Create backup (DB only) | ZIP with database created | backup.rs |
| FT-37 | Backup | Create backup (with files) | ZIP with DB + referenced files | backup.rs |
| FT-38 | Backup | Restore backup | DB restored from ZIP | backup.rs restore_backup |
| FT-39 | Projects | CRUD projects | Create, read, update, delete, duplicate | projects commands |
| FT-40 | Projects | Collections | Create, add/remove files, delete collection | projects commands |
| FT-41 | Manufacturing | Supplier CRUD | All vendor operations | manufacturing commands |
| FT-42 | Manufacturing | Material inventory tracking | Stock levels maintained | manufacturing commands |
| FT-43 | Manufacturing | Product variants | Size/color/customization variants | manufacturing commands |
| FT-44 | Manufacturing | Bill of Materials | BOM entries CRUD | manufacturing commands |
| FT-45 | Manufacturing | Workflow steps | Step definitions and workflow tracking | manufacturing commands |
| FT-46 | Manufacturing | Material reservation/consumption | Reserve for project, record usage | manufacturing commands |
| FT-47 | Manufacturing | Quality inspections/defects | Create and track quality records | manufacturing commands |
| FT-48 | Procurement | Purchase orders CRUD | Order lifecycle management | procurement commands |
| FT-49 | Procurement | Order items and deliveries | Line items and delivery tracking | procurement commands |
| FT-50 | Procurement | Order suggestions | Auto-suggest based on requirements | procurement commands |
| FT-51 | Reports | Project report generation | Cost breakdown, material usage | reports commands |
| FT-52 | Reports | CSV exports (BOM, orders, materials) | Valid CSV files generated | reports commands |
| FT-53 | File Watcher | Start/stop watcher | Watcher lifecycle management | file_watcher.rs |
| FT-54 | File Watcher | Auto-import on file creation | New files detected and imported | file_watcher.rs |
| FT-55 | File Watcher | File removal detection | Removed files signaled | file_watcher.rs |
| FT-56 | File Watcher | Debounce (500ms) | Rapid events coalesced | file_watcher.rs DEBOUNCE_MS |
| FT-57 | Print | PDF report generation | Valid PDF created | print commands |
| FT-58 | Print | Tile computation | Page tiling calculated correctly | print commands |
| FT-59 | Attachments | File attachment CRUD | Attach, list, delete, open | file commands |
| FT-60 | Versioning | Version history | Create, list, restore, delete versions | versioning commands |
| FT-61 | Audit | Change history logging | All entity changes recorded | audit commands |
| FT-62 | UI - Virtual scroll | Large file list rendering | Only visible cards rendered (CARD_HEIGHT=72) | FileList.ts |
| FT-63 | UI - Keyboard | All shortcuts functional | Escape, Ctrl+S/F/P/,, Delete, arrows | shortcuts.ts |
| FT-64 | UI - Dialogs | Focus trap in all dialogs | Tab cycling within dialog, restore on close | focus-trap.ts |
| FT-65 | UI - Toast | Notification system | Max 5, auto-dismiss 4s, all types | Toast.ts |
| FT-66 | UI - Splitter | Panel resize | Drag dividers, persist widths | Splitter.ts |
| FT-67 | UI - Metadata | Dirty tracking | Unsaved changes detected, save/discard | MetadataPanel.ts |

### 3.2 Performance Tests (PT)

| ID | Category | Test | Metric | Threshold |
|----|----------|------|--------|-----------|
| PT-01 | Virtual scroll | Render 10,000 files | DOM nodes | < 100 visible + 10 buffer |
| PT-02 | Virtual scroll | Scroll through 10K files | FPS | > 30 FPS |
| PT-03 | DB query | get_files with FTS5 search | Response time | < 200ms for 10K records |
| PT-04 | DB query | Advanced search (5+ filters) | Response time | < 500ms for 10K records |
| PT-05 | Batch rename | 1000 files | Total time | < 30s |
| PT-06 | Batch organize | 1000 files | Total time | < 60s |
| PT-07 | File import | scan_directory with 500 files | Total time | < 120s |
| PT-08 | Thumbnail gen | 100 stitch renders | Total time | < 60s |
| PT-09 | Memory | App idle after loading 10K files | RSS memory | < 500MB |
| PT-10 | Memory | Subscription cleanup on HMR | Leak detection | No growth after 10 cycles |
| PT-11 | DB lock contention | Concurrent read + write | Deadlock | No deadlocks, < 5s busy_timeout |
| PT-12 | Search debounce | Rapid typing (20 chars/sec) | Backend calls | 1 call per 300ms max |
| PT-13 | File watcher | 100 rapid file changes | Events emitted | Coalesced to < 10 events |
| PT-14 | Thumbnail cache | 200 cache entries | Cache eviction | LRU eviction at THUMB_CACHE_MAX |
| PT-15 | Startup | App cold start | Time to interactive | < 5s |

### 3.3 Security Tests (ST)

Based on OWASP Top 10 (2021), CWE/SANS Top 25, and Tauri-specific vectors.

| ID | Standard | Category | Test | Severity |
|----|----------|----------|------|----------|
| **Injection (OWASP A03)** | | | | |
| ST-01 | CWE-89 | SQL Injection | FTS5 query with special chars: `"`, `*`, `(`, `)`, `{`, `}`, `:` | Critical |
| ST-02 | CWE-89 | SQL Injection | LIKE query with `%`, `_`, `\` characters | Critical |
| ST-03 | CWE-89 | SQL Injection | Dynamic ORDER BY clause validation | Critical |
| ST-04 | CWE-89 | SQL Injection | Parameterized queries in all 155+ commands | Critical |
| ST-05 | CWE-79 | XSS | innerHTML usage audit (all 60+ instances) | High |
| ST-06 | CWE-79 | XSS | Template literal injection in HTML construction | High |
| ST-07 | CWE-79 | XSS | User data in DOM attributes (dataset, aria-*) | Medium |
| ST-08 | CWE-94 | Code Injection | eval()/Function() usage scan | Critical |
| ST-09 | CWE-78 | OS Command Injection | Shell execute via Tauri opener | High |
| **Broken Access Control (OWASP A01)** | | | | |
| ST-10 | CWE-22 | Path Traversal | `..` in file paths, batch patterns, folder creation | Critical |
| ST-11 | CWE-22 | Path Traversal | Unicode normalization bypasses (`..\u2025`) | High |
| ST-12 | CWE-284 | Access Control | Tauri capability restrictions (principle of least privilege) | Medium |
| ST-13 | CWE-862 | Missing Auth | Command access without authentication | Medium |
| **Cryptographic Failures (OWASP A02)** | | | | |
| ST-14 | CWE-312 | Plaintext Secrets | API keys stored unencrypted in SQLite | High |
| ST-15 | CWE-319 | Cleartext Transmission | AI API keys in HTTP headers | Medium |
| ST-16 | CWE-327 | Weak Crypto | SHA2 usage for hashing (is it sufficient?) | Low |
| **Security Misconfiguration (OWASP A05)** | | | | |
| ST-17 | CWE-16 | CSP | `unsafe-inline` in style-src directive | Medium |
| ST-18 | CWE-16 | CSP | Missing connect-src, form-action, frame-ancestors | Medium |
| ST-19 | CWE-200 | Info Disclosure | Error messages expose internal paths/state | Medium |
| ST-20 | CWE-532 | Log Injection | Log messages contain user-controlled data | Low |
| **Vulnerable Components (OWASP A06)** | | | | |
| ST-21 | CWE-1104 | Dependencies | `cargo audit` — known Rust CVEs | High |
| ST-22 | CWE-1104 | Dependencies | `npm audit` — known Node CVEs | High |
| ST-23 | CWE-1104 | Dependencies | Bundled SQLite version CVEs | Medium |
| **Data Integrity (OWASP A08)** | | | | |
| ST-24 | CWE-367 | TOCTOU | Batch operation race conditions (check-then-act) | Medium |
| ST-25 | CWE-362 | Race Condition | Mutex poisoning under concurrent commands | Medium |
| ST-26 | CWE-20 | Input Validation | File size limits, name length limits, field validation | Medium |
| **Tauri-Specific** | | | | |
| ST-27 | — | IPC Security | Tauri invoke() command surface audit | High |
| ST-28 | — | Window Security | Window decorations, fullscreen, resizable | Low |
| ST-29 | — | Event System | Backend event emission — can frontend spoof events? | Medium |
| ST-30 | — | File Dialog | Dialog API restrictions vs arbitrary file access | Medium |
| **Additional CWE/SANS** | | | | |
| ST-31 | CWE-400 | Resource Exhaustion | Unbounded batch operations, large file uploads | Medium |
| ST-32 | CWE-502 | Deserialization | serde_json parsing of AI responses | Medium |
| ST-33 | CWE-611 | XXE | XML parsing in thread color files (roxmltree) | Medium |
| ST-34 | CWE-770 | Allocation | Thumbnail generation memory for large files | Medium |
| ST-35 | CWE-798 | Hardcoded Creds | Source code scan for secrets/tokens | High |

---

## 4. Execution Strategy

### Agent Assignment

| Phase | Agent 1 (Claude) | Agent 2 (Codex) |
|-------|------------------|-----------------|
| Functional Tests | Execute FT-01..FT-67 | Independent verification of FT-01..FT-67 |
| Performance Tests | Execute PT-01..PT-15 | Independent verification of PT-01..PT-15 |
| Security Tests | Execute ST-01..ST-35 | Independent verification of ST-01..ST-35 |
| Cross-validation | Validate Codex findings | Validate Claude findings |

### Validation Protocol

1. Each agent executes tests independently
2. Findings are compared; discrepancies require re-testing
3. **Only cross-validated findings** become GitHub issues
4. Each issue includes:
   - Test ID reference (FT-XX, PT-XX, ST-XX)
   - Detailed description
   - Reproduction steps / code reference
   - Severity (Critical/High/Medium/Low)
   - Proposed solution
   - Affected files

### Output Structure

```
release_26.04-a1/
├── test-plan.md                    # This document (copied)
├── claude-functional-report.md     # Claude agent functional test results
├── claude-performance-report.md    # Claude agent performance test results
├── claude-security-report.md       # Claude agent security test results
├── codex-functional-report.md      # Codex agent functional test results
├── codex-performance-report.md     # Codex agent performance test results
├── codex-security-report.md        # Codex agent security test results
├── cross-validation.md             # Cross-validation results
├── findings-summary.md             # All validated findings
└── final-report.md                 # Release test report
```

---

## 5. Proposed Approach

### Step 1: Execute Rust tests (`cargo test`) as baseline
### Step 2: Execute TypeScript build (`npm run build`) as baseline
### Step 3: Run `cargo audit` and `npm audit` for dependency vulnerabilities
### Step 4: Launch two independent agents for code-level testing:
- **Claude agent**: Static analysis + code review of all test categories
- **Codex agent**: Independent static analysis + code review
### Step 5: Cross-validate findings between agents
### Step 6: Create GitHub issues for all validated findings
### Step 7: Compile final test report in `release_26.04-a1/`

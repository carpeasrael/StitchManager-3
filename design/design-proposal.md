# StichMan v2 — Design Proposal

> Reverse-engineered from `StichMan.app` v0.4-beta · Author: CarpeAsrael
> Proposal date: 2026-03-05

---

## 1. Executive Summary

This document formalises the visual design for StichMan v2. The app already ships a functional Aurora Light / Dark theme system with design tokens, but no design documentation or component specifications exist. This proposal covers:

- The full design-token set (light + dark, semantic naming)
- A typography and spacing system
- A component inventory with visual specs
- Main-window layout improvements over v0.4
- Dialog design outlines
- A reference mockup (`mockup.svg`, 1440 × 900 px, light mode)

No application code is changed by this document. It serves as a handoff reference for v2 implementation.

---

## 2. Design Principles

### 2.1 Craft-first
The UI should feel as refined as the embroidery it manages. Generous whitespace, crisp typography, and consistent micro-details signal quality to craft-focused users.

### 2.2 Information density without overwhelm
Embroidery files carry rich metadata (dimensions, stitch count, thread colours, tags, AI fields). All of it must be visible without scrolling in the metadata panel — use visual hierarchy (label weight, colour, size) to let users scan quickly.

### 2.3 Progressive disclosure
AI features appear only when relevant. The KI Analyse button is present but visually secondary. AI-generated content is clearly labelled so users know what was human-edited vs. machine-suggested.

### 2.4 Accessibility baseline
All text/background pairs must meet WCAG AA (4.5:1 for body text, 3:1 for large text). The token validator already enforces this for custom themes — apply the same rule to built-in palettes in code review.

---

## 3. Design System

### 3.1 Color Palette

#### Aurora Light (theme_mode = "hell")

| Token | Hex | Role |
|---|---|---|
| `bg` | `#f5f5f7` | Window background, sidebar, centre panel |
| `surface` | `#ffffff` | Cards, inputs, right panel, toolbar |
| `elevated` | `#ffffff` | Dialogs, popovers |
| `text` | `#111111` | Primary text |
| `text-secondary` | `#44474f` | Secondary labels, menu items |
| `muted` | `#7b7c80` | Placeholders, section headers, status text |
| `muted-light` | `#b4b7bd` | Ghost placeholders, disabled |
| `accent` | `#0a84ff` | Buttons, selections, links, active chips |
| `accent-strong` | `#086dd6` | Pressed/hover accent, selected folder text |
| `accent-10` | `#e8f2ff` | Chip backgrounds, tag backgrounds |
| `accent-20` | `#cee6ff` | Selected list item background |
| `border` | `#d1d5db` | Panel dividers, input outlines, separators |
| `border-light` | `#e5e7eb` | Card borders, subtle dividers |
| `status-green` | `#51cf66` | AI analysed indicator dot |
| `status-green-bg` | `#dcfce7` | AI analysed badge background |
| `status-green-text` | `#2f9e44` | AI analysed badge text |
| `status-red` | `#ff6b6b` | Error / warning |

#### Aurora Dark (theme_mode = "dunkel")

| Token | Hex | Role |
|---|---|---|
| `bg` | `#0f0f10` | Window background |
| `surface` | `#1f1f23` | Cards, inputs |
| `elevated` | `#242428` | Dialogs, toolbar |
| `text` | `#f5f5f7` | Primary text |
| `text-secondary` | `#a0a3ab` | Secondary labels |
| `muted` | `#5c5e63` | Placeholders |
| `accent` | `#2d7ff9` | Buttons, links |
| `accent-strong` | `#4a94ff` | Hover state |
| `border` | `#2e2e35` | Dividers |
| `border-light` | `#27272e` | Card borders |

---

### 3.2 Typography Scale

Font stack: `"Helvetica Neue", "Segoe UI", Helvetica, Arial, sans-serif`

| Level | Size | Weight | Usage |
|---|---|---|---|
| Display | 20 px | 700 | Window title, large headings |
| Heading | 15 px | 600 | Panel section titles |
| Body | 13 px | 400 | List items, form values, general text |
| Label | 13 px | 500 | Form labels inline |
| Section Header | 10–11 px | 700, uppercase, letter-spacing 0.8 px | "ORDNER", "NAME", "FARBEN" |
| Caption | 10–11 px | 400 | Badge text, status bar, helper text |

---

### 3.3 Spacing Scale

Base grid: **4 px**

| Step | Value | Usage |
|---|---|---|
| 1 | 4 px | Icon inner padding, chip vertical padding |
| 2 | 8 px | Gap between form sections (tight) |
| 3 | 12 px | Card padding, search bar h-padding |
| 4 | 16 px | Panel inner padding (sides) |
| 5 | 20 px | Panel inner padding (right panel) |
| 6 | 24 px | Between major sections |
| 8 | 32 px | Panel minimum column gap |
| 12 | 48 px | Toolbar height |

---

### 3.4 Border Radius Scale

| Context | Value |
|---|---|
| Inputs, list item hover backgrounds | 6 px |
| Cards, preview images | 8 px |
| Dialogs, panels | 12 px |
| Toolbar buttons | 8 px |
| Filter chips, tag chips, badges | 999 px (pill) |
| Color swatches | 4 px |

---

### 3.5 Shadow / Elevation Scale

| Level | CSS equivalent | Usage |
|---|---|---|
| None | — | Flat inputs, sidebar items |
| xs | `0 1px 3px rgba(0,0,0,0.06)` | File list cards (resting) |
| sm | `0 2px 6px rgba(0,0,0,0.10)` | Cards on hover |
| md | `0 4px 16px rgba(0,0,0,0.12)` | Dialogs, popovers |

---

### 3.6 Icon Grid

All icons: **24 × 24 px**, stroke-only, `stroke-width: 2`, `stroke-linecap: round`, `stroke-linejoin: round`, no fill.

Existing icons in bundle: `brain`, `file-plus`, `folder-plus`, `settings`, `usb`

Recommended additions for v2: `search`, `chevron-right`, `chevron-left`, `check`, `x`, `save`, `tag`, `image`, `copy`, `trash`

Toolbar icons render at **16 × 16 px** (scaled down, same grid proportions).

---

## 4. Component Inventory

### 4.1 Sidebar Folder Item

| Property | Resting | Hover | Selected |
|---|---|---|---|
| Background | transparent | `accent-10` | `accent-20` |
| Border radius | 6 px | 6 px | 6 px |
| Padding | 8 px h · 6 px v | same | same |
| Text color | `text-secondary` | `text` | `accent-strong` |
| Text weight | 400 | 500 | 600 |
| Icon stroke | `muted` | `text-secondary` | `accent` |

### 4.2 File List Mini-Card

- Background: `surface`
- Border: 1 px `border-light`
- Border radius: 8 px
- Shadow: xs (resting), sm (hover)
- Height: 72 px
- Layout: thumbnail (48 × 48, radius 6) | name + format chips + AI badge
- Selected state: background `accent-10`, border 1.5 px `accent`

### 4.3 Search Bar

- Background: `surface`
- Border: 1 px `border`
- Border radius: 999 px (pill)
- Height: 32 px
- Left icon: magnifier 16 × 16, stroke `muted`
- Right: clear × icon, visible only when input non-empty
- Placeholder color: `muted-light`

### 4.4 Filter Chip

| State | Background | Border | Text |
|---|---|---|---|
| Inactive | transparent | 1 px `border` | `text-secondary` |
| Active | `accent` | none | white |
| Hover (inactive) | `accent-10` | 1 px `border` | `text` |

- Border radius: 999 px
- Height: 24 px
- Padding: 0 12 px

### 4.5 Tag Chip (display, inside input)

- Background: `accent-10`
- Text: `accent-strong`, 11 px
- Border radius: 999 px
- Height: 20 px
- Padding: 0 8 px

### 4.6 AI Status Badge

| State | Background | Dot | Text |
|---|---|---|---|
| Analysiert | `status-green-bg` | `status-green` (4 px circle) | `status-green-text` |
| Nicht analysiert | `bg` | none | `muted` |

- Border radius: 999 px
- Height: 22–24 px

### 4.7 Format Chip (PES / DST etc.)

- Background: `accent-10`
- Text: `accent-strong`, 10–11 px, monospace or bold
- Border radius: 999 px
- Height: 16–20 px

### 4.8 Color Swatch

- Size: 22 × 22 px
- Border radius: 4 px
- Border: 1 px `border` (ensures white swatches are visible)
- Tooltip: thread name on hover

### 4.9 Primary Button

- Background: `accent`
- Text: white, 13 px, weight 500
- Border radius: 8 px
- Padding: 8 px × 16 px (min-width via padding)
- Hover: `accent-strong`
- Pressed: darken 8%

### 4.10 Ghost Button

- Background: transparent
- Border: 1 px `border`
- Text: `text-secondary`
- Border radius: 8 px
- Hover: background `bg`

### 4.11 Accent Ghost Button (KI Analyse)

- Background: transparent
- Border: 1 px `accent`
- Text: `accent`
- Hover: background `accent-10`

### 4.12 Form Input

- Background: `bg`
- Border: 1 px `border-light`
- Border radius: 6 px
- Height: 32 px (single-line), 64 px (multiline description)
- Focus: border `accent`, box-shadow `0 0 0 3px rgba(10,132,255,0.15)`
- Text: `text`, 13 px

### 4.13 Preview Image

- Width: 100% of right panel content area
- Height: 190 px (fixed in non-scrolling layout)
- Border radius: 8 px
- Background: `bg` (shown while loading)
- Object-fit: contain

### 4.14 Toolbar Button

- Icon: 16 × 16 px, stroke `text-secondary`
- Label: 12 px, `text-secondary`, to the right of icon
- Hover: background rounded rect, fill `bg`
- Active AI button: icon stroke `accent`, label color `accent`

---

## 5. Main Window Layout Proposal

### 5.1 Current layout (v0.4)

```
[Menu bar]
[Sidebar: folders] | [Centre: file list + filters] | [Right: metadata form]
[Status bar]
```

### 5.2 Proposed layout (v2)

```
[Menu bar          h=28]
[Toolbar           h=48]
[Sidebar w=240] | [Centre w=480] | [Right w=720]   h=802
[Status bar        h=22]
```

Total: 1440 × 900 px. Panels are resizable via splitter handles.

### 5.3 Improvements over v0.4

| Area | Change |
|---|---|
| Toolbar | Add dedicated toolbar with icon buttons above the 3-panel area (Add File, Add Folder, Save, Settings, KI Analyse). This surfaces primary actions that are currently hidden in the menu. |
| Sidebar | Add "+" button pinned to bottom. Add collapsible toggle (chevron button). Show file count per folder as a muted badge on the right. |
| File list | Replace flat list with mini-cards (thumbnail + name + format chips + AI badge). Add per-card hover shadow. Visually distinguish selected card with accent border. |
| Search | Move to fixed top bar spanning the full centre panel width. Pill shape for modern feel. |
| Filter chips | Horizontal scroll strip below search. Include format chips (PES, DST…) and tag chips. |
| Metadata panel | Add a status strip pinned to the bottom: file size, format badges, version. Separate action buttons visually from form fields. |
| AI fields | Show "(KI-generiert)" caption below AI-filled fields until user confirms them. |

### 5.4 Panel width guidelines

| Panel | Default | Min | Max |
|---|---|---|---|
| Sidebar | 240 px | 160 px | 320 px |
| Centre | 480 px | 300 px | 600 px |
| Right | remainder | 480 px | — |

---

## 6. Dialog Designs

### 6.1 Settings Dialog

Tabbed layout, 720 × 560 px:

| Tab | Key controls |
|---|---|
| Allgemein | Library root path picker, metadata root path picker |
| Erscheinungsbild | Theme selector (hell / dunkel / OS / custom), theme preview swatch |
| KI | Provider toggle (Ollama / OpenAI), URL, API key, model, temperature, timeout |
| Dateiverwaltung | Rename pattern, Organize pattern, pattern history dropdown |
| Benutzerdefiniert | Custom field list with add/remove/reorder, required flag toggle |

### 6.2 AI Preview Dialog (AiPreviewDialog)

Split view, 800 × 600 px:

- Left pane: Prompt text preview (scrollable, monospace, read-only)
- Right pane: Metadata summary card (file name, image thumbnail, fields to be sent)
- Bottom: "Senden" (primary) / "Abbrechen" (ghost) buttons

### 6.3 AI Result Dialog (AiResultDialog)

Card-per-field layout, 640 × 500 px:

- Each field: checkbox (accept?) + field name label + editable text box
- Colour fields: AiColorDialog sub-view with original vs. AI name side by side
- Bottom: "Übernehmen" (primary) / "Verwerfen" (ghost)

### 6.4 Batch Process Dialog (BatchProcessProgressDialog)

- Step indicators (Rename → Organize)
- Per-step progress bar
- Scrollable log view
- "Abbrechen" button (disabled once complete)

---

## 7. Mockup Key

The file `mockup.svg` in this directory shows the main window in **Aurora Light** mode at **1440 × 900 px**.

### Regions

| Region | x | y | w | h |
|---|---|---|---|---|
| Menu bar | 0 | 0 | 1440 | 28 |
| Toolbar | 0 | 28 | 1440 | 48 |
| Sidebar | 0 | 76 | 240 | 802 |
| Centre panel | 240 | 76 | 480 | 802 |
| Right panel | 720 | 76 | 720 | 802 |
| Status bar | 0 | 878 | 1440 | 22 |

### State shown in mockup

- Selected folder: **Weihnachten**
- Selected file: **Weihnachts-Rentier.pes** (card 3 in centre, full metadata in right panel)
- AI status: analysiert (llama3.2-vision)
- Active filter chip: **Alle**
- 6 file cards visible; cards 1 and 4 show "Nicht analysiert" state

### Colour fidelity

All hex values in the mockup are taken directly from the extracted Aurora Light palette. No colour has been approximated or invented.

---

*End of design proposal.*

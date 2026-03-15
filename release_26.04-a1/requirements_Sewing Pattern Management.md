# User Requirements for a Sewing Pattern Management App

## 1. Purpose

The application shall enable users to store, organize, view, search, manage, and print sewing patterns and their related instructions in a structured and user-friendly way. The app shall support hobby users as well as professional users with large pattern libraries.

## 2. Scope

The application shall manage:

* sewing pattern files
* sewing instruction documents
* metadata related to patterns
* preview images
* categories, tags, and filters
* direct printing of sewing patterns from within the app


# 4. User Requirements

## 4.1 General Requirements

**UR-001**
The app shall allow users to create and maintain a digital library of sewing patterns.

**UR-002**
The app shall allow users to store one or more files per sewing pattern entry.

**UR-003**
The app shall allow users to link sewing instructions to the corresponding sewing pattern.

**UR-004**
The app shall allow users to manage patterns and instructions in a single unified interface.

**UR-005**
The app shall support large collections without making navigation impractical.

**UR-006**
The app shall provide a clear and simple user interface suitable for frequent use.

---

## 4.2 Pattern Import and Storage

**UR-007**
The app shall allow users to import sewing pattern files from local storage.

**UR-008**
The app shall support common sewing-related file formats, including at least PDF for printable patterns and standard image/document formats for associated content.

**UR-009**
The app should allow users to attach multiple related files to one pattern record, for example:

* pattern PDF
* instructions PDF
* cover image
* measurement chart
* fabric requirements sheet

**UR-010**
The app shall preserve the original imported files without modifying them unless the user explicitly performs an edit/export action.

**UR-011**
The app should allow drag-and-drop import of files.

**UR-012**
The app should support bulk import of multiple patterns at once.

---

## 4.3 Metadata Management

**UR-013**
The app shall allow the user to enter and edit metadata for each sewing pattern.

**UR-014**
The metadata should include at least:

* pattern title
* designer/brand
* garment/project type
* size range
* skill level
* language
* format type
* file source
* purchase/source link
* notes

**UR-015**
The app should allow users to define custom metadata fields.

**UR-016**
The app shall allow users to assign tags to patterns.

**UR-017**
The app shall allow patterns to be grouped into categories or collections.

**UR-018**
The app should allow users to record status information, such as:

* not started
* planned
* in progress
* completed
* archived

**UR-019**
The app should allow users to store project-specific notes separately from the original pattern metadata.

---

## 4.4 Instruction Management

**UR-020**
The app shall allow users to attach sewing instructions to a pattern entry.

**UR-021**
The app shall allow users to open and read instructions inside the application, where technically feasible.

**UR-022**
The app should display instructions in a readable format optimized for screen viewing.

**UR-023**
The app should allow users to navigate instructions page by page or section by section.

**UR-024**
The app should allow users to add notes or comments linked to the instructions.

**UR-025**
The app should allow users to mark important instruction pages or sections as favorites/bookmarks.

---

## 4.5 Search, Filter, and Browse

**UR-026**
The app shall provide a searchable list of all stored sewing patterns.

**UR-027**
The app shall allow users to search by metadata fields, tags, and free text.

**UR-028**
The app shall allow filtering by relevant criteria such as:

* garment type
* size
* skill level
* designer/brand
* language
* status
* tags

**UR-029**
The app shall allow sorting by title, date added, designer, category, and last modified date.

**UR-030**
The app should provide thumbnail or preview-based browsing for easier visual identification.

---

## 4.6 Preview and Viewing

**UR-031**
The app shall allow the user to preview sewing patterns before printing.

**UR-032**
The app shall allow the user to preview instructions before opening or printing them.

**UR-033**
The app should support zooming and panning for pattern preview.

**UR-034**
The app should display page count, paper size, and document properties for printable patterns where available.

**UR-035**
The app should allow switching between single-page view and multi-page overview for pattern documents.

---

## 4.7 Direct Printing of Sewing Patterns

**UR-036**
The app shall allow sewing patterns to be printed directly from within the application.

**UR-037**
The user shall not be required to open an external application in order to print a stored sewing pattern.

**UR-038**
The app shall provide a print preview before printing.

**UR-039**
The app shall allow printing of the full sewing pattern or selected pages only.

**UR-040**
The app shall allow the user to choose paper size, orientation, page range, and printer.

**UR-041**
The app shall ensure that printable sewing patterns can be printed at **true scale**.

**UR-042**
The app shall prevent unintended scaling by default for pattern printing, unless the user explicitly selects a scaling option.

**UR-043**
The app should provide a visible warning if print settings may alter scale accuracy.

**UR-044**
The app should support common home printing formats such as A4 and US Letter.

**UR-045**
The app should support tiled multi-page printing for large sewing patterns.

**UR-046**
The app should support printing of copy-shop or large-format pattern pages if such files are available.

**UR-047**
The app should allow users to print only selected layers or views if the pattern format supports layered printing.

**UR-048**
The app should display a measurement calibration element or test square in preview where present in the source file.

**UR-049**
The app should preserve line clarity and readability when printing pattern pieces, markings, and labels.

**UR-050**
The app may allow printing instructions directly from within the application as well.

---

## 4.8 Organization and Project Use

**UR-051**
The app should allow users to mark favorite patterns.

**UR-052**
The app should allow users to create project folders or project entries linked to a sewing pattern.

**UR-053**
The app should allow users to store project-specific information such as:

* chosen size
* fabric used
* planned modifications
* cut version
* sewing notes

**UR-054**
The app should allow users to duplicate a project from the same sewing pattern without duplicating the original source files.

---

## 4.9 File Integrity and Data Protection

**UR-055**
The app shall keep references between patterns and instructions intact.

**UR-056**
The app shall protect against accidental deletion of records or linked files by providing a confirmation step.

**UR-057**
The app should provide a recycle bin, archive function, or recovery option for deleted entries.

**UR-058**
The app should support backup and restore of the pattern library and metadata.

**UR-059**
The app should avoid data loss in case of unexpected shutdown.

---

## 4.10 Import/Export and Portability

**UR-060**
The app should allow export of metadata and library information.

**UR-061**
The app should allow users to move or migrate their collection to another device without rebuilding the library manually.

**UR-062**
The app should support export of selected pattern records including metadata and linked file references.

**UR-063**
The app should support re-linking missing files if storage paths have changed.

---

## 4.11 Usability Requirements

**UR-064**
The app shall be usable by non-technical users.

**UR-065**
The app should minimize the number of steps required to find and print a sewing pattern.

**UR-066**
The app should provide a clear distinction between:

* original pattern files
* instructions
* project notes
* printed output settings

**UR-067**
The app should provide responsive navigation even for large libraries.

**UR-068**
The app should support dark mode and light mode.

---

## 4.12 Performance Requirements

**UR-069**
The app shall open a stored sewing pattern within an acceptable time for normal use.

**UR-070**
The app shall remain stable when managing large numbers of patterns and associated documents.

**UR-071**
The app should support bulk operations without major degradation of usability.

---

## 4.13 Optional Multi-User / Advanced Requirements

**UR-072**
If multi-user functionality is implemented, the app shall allow separate user profiles or accounts.

**UR-073**
If multi-user functionality is implemented, the app should allow role-based permissions for editing, deleting, and printing.

**UR-074**
If cloud or shared storage is implemented, the app should handle conflicts in a controlled manner.

---

# 5. Key Acceptance Expectations

The following are especially critical for acceptance:

**AE-001**
A user can import a sewing pattern and its instructions into one app record.

**AE-002**
A user can search and retrieve a pattern by title, tag, category, or metadata.

**AE-003**
A user can open the instructions from the same record as the sewing pattern.

**AE-004**
A user can preview a sewing pattern before printing.

**AE-005**
A user can print the pattern directly from the app without using an external viewer.

**AE-006**
The printed output preserves correct scale when printed using default print settings.

**AE-007**
A user can print selected pages only.

**AE-008**
A user can manage a growing library of patterns without losing overview.

---

# 6. Priority Recommendation

## Must Have

* library management for patterns and instructions
* metadata and tagging
* search and filter
* preview
* direct printing from the app
* true-scale printing control
* support for common printable formats

## Should Have

* bulk import
* thumbnails
* project notes
* backups
* export/migration
* favorites and collections

## Could Have

* layered printing
* multi-user mode
* cloud sync
* custom fields
* advanced project tracking



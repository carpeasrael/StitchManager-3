import { Component } from "./Component";
import { appState } from "../state/AppState";
import { EventBus } from "../state/EventBus";
import { ToastContainer } from "./Toast";
import { getFormatLabel, formatSize } from "../utils/format";
import * as FileService from "../services/FileService";

const CARD_HEIGHT = 72;
const BUFFER = 5;
const THUMB_CACHE_MAX = 200;
const PAGE_SIZE = 500;

export class FileList extends Component {
  private generation = 0;
  private lastClickedIndex: number | null = null;
  private listEl: HTMLElement | null = null;
  private spacer: HTMLElement | null = null;
  private scrollContainer: HTMLElement | null = null;
  private visibleStart = 0;
  private visibleEnd = 0;
  private scrollRafPending = false;
  private thumbCache = new Map<number, string>();
  private renderedCards = new Map<number, HTMLElement>();
  private currentPage = 0;
  private totalCount = 0;
  private loadingMore = false;

  constructor(container: HTMLElement) {
    super(container);
    this.subscribe(
      appState.on("selectedFolderId", () => this.loadFiles())
    );
    this.subscribe(
      appState.on("searchQuery", () => this.loadFiles())
    );
    this.subscribe(
      appState.on("searchParams", () => this.loadFiles())
    );
    this.subscribe(
      appState.on("formatFilter", () => this.loadFiles())
    );
    this.subscribe(
      appState.on("files", () => this.render())
    );
    this.subscribe(
      appState.on("selectedFileId", () => this.updateSelection())
    );
    this.subscribe(
      appState.on("selectedFileIds", () => this.updateSelection())
    );
    this.subscribe(
      EventBus.on("filelist:scroll-to-index", (index: unknown) => {
        this.scrollToIndex(index as number);
      })
    );
    this.loadFiles();
  }

  private async loadFiles(): Promise<void> {
    const gen = ++this.generation;
    this.currentPage = 0;
    this.totalCount = 0;
    const folderId = appState.get("selectedFolderId");
    const search = appState.get("searchQuery");
    const formatFilter = appState.get("formatFilter");
    const searchParams = appState.get("searchParams");

    try {
      const result = await FileService.getFilesPaginated(folderId, search, formatFilter, searchParams, 0, PAGE_SIZE);
      if (gen !== this.generation) return;
      this.totalCount = result.totalCount;
      this.currentPage = 0;
      appState.set("files", result.files);
    } catch (e) {
      console.warn("Failed to load files:", e);
      ToastContainer.show("error", "Dateien konnten nicht geladen werden");
    }
  }

  private async loadMoreFiles(): Promise<void> {
    const files = appState.getRef("files");
    if (this.loadingMore || files.length >= this.totalCount) return;
    this.loadingMore = true;
    const gen = this.generation;
    const folderId = appState.get("selectedFolderId");
    const search = appState.get("searchQuery");
    const formatFilter = appState.get("formatFilter");
    const searchParams = appState.get("searchParams");

    try {
      const nextPage = this.currentPage + 1;
      const result = await FileService.getFilesPaginated(folderId, search, formatFilter, searchParams, nextPage, PAGE_SIZE);
      if (gen !== this.generation) return;
      if (result.files.length > 0) {
        this.currentPage = nextPage;
        this.totalCount = result.totalCount;
        const existing = appState.getRef("files");
        appState.set("files", [...existing, ...result.files]);
      }
    } catch (e) {
      console.warn("Failed to load more files:", e);
    } finally {
      this.loadingMore = false;
    }
  }

  render(): void {
    const files = appState.getRef("files");
    this.lastClickedIndex = null;
    this.thumbCache.clear();
    this.renderedCards.clear();

    this.el.innerHTML = "";

    if (files.length === 0) {
      const empty = document.createElement("div");
      empty.className = "file-list-empty";
      empty.textContent = "Keine Dateien gefunden";
      this.el.appendChild(empty);
      this.listEl = null;
      this.spacer = null;
      this.scrollContainer = null;
      return;
    }

    // Scroll container for virtual scrolling
    this.scrollContainer = document.createElement("div");
    this.scrollContainer.className = "file-list";
    this.scrollContainer.setAttribute("role", "list");
    this.scrollContainer.setAttribute("aria-label", "Dateien");
    this.scrollContainer.addEventListener("scroll", () => this.onScroll());

    // Spacer sets total height for scrollbar accuracy
    this.spacer = document.createElement("div");
    this.spacer.style.height = `${files.length * CARD_HEIGHT}px`;
    this.spacer.style.position = "relative";

    this.listEl = this.spacer;
    this.scrollContainer.appendChild(this.spacer);
    this.el.appendChild(this.scrollContainer);

    this.calculateVisibleRange();
    this.renderVisible();
  }

  private onScroll(): void {
    if (this.scrollRafPending) return;
    this.scrollRafPending = true;
    requestAnimationFrame(() => {
      this.scrollRafPending = false;
      const oldStart = this.visibleStart;
      const oldEnd = this.visibleEnd;
      this.calculateVisibleRange();

      if (this.visibleStart !== oldStart || this.visibleEnd !== oldEnd) {
        this.renderVisible();
      }

      // Load more files when approaching the end of the loaded list
      const files = appState.getRef("files");
      if (files.length < this.totalCount && this.visibleEnd >= files.length - BUFFER * 2) {
        this.loadMoreFiles();
      }
    });
  }

  private calculateVisibleRange(): void {
    if (!this.scrollContainer) return;
    const files = appState.getRef("files");
    const scrollTop = this.scrollContainer.scrollTop;
    const containerHeight = this.scrollContainer.clientHeight;

    const start = Math.floor(scrollTop / CARD_HEIGHT);
    const visibleCount = Math.ceil(containerHeight / CARD_HEIGHT);

    this.visibleStart = Math.max(0, start - BUFFER);
    this.visibleEnd = Math.min(files.length, start + visibleCount + BUFFER);
  }

  private renderVisible(): void {
    if (!this.listEl) return;
    const files = appState.getRef("files");
    const selectedId = appState.getRef("selectedFileId");
    const selectedIds = appState.getRef("selectedFileIds");

    // Update spacer height in case file count changed
    this.listEl.style.height = `${files.length * CARD_HEIGHT}px`;

    // Remove cards outside the visible range
    for (const [index, card] of this.renderedCards) {
      if (index < this.visibleStart || index >= this.visibleEnd) {
        card.remove();
        this.renderedCards.delete(index);
      }
    }

    // Add cards that entered the visible range
    const newFileIds: number[] = [];
    for (let i = this.visibleStart; i < this.visibleEnd; i++) {
      if (this.renderedCards.has(i)) continue;
      const file = files[i];
      if (!file) continue;

      const card = this.createCard(file, i, selectedId, selectedIds);
      this.listEl.appendChild(card);
      this.renderedCards.set(i, card);
      newFileIds.push(file.id);
    }

    // Batch-load thumbnails for newly rendered cards
    const uncachedIds = newFileIds.filter((id) => !this.thumbCache.has(id));
    if (uncachedIds.length > 0) {
      FileService.getThumbnailsBatch(uncachedIds).then((thumbs) => {
        // Build file-ID-to-card map for O(1) lookups
        const cardsByFileId = new Map<number, HTMLElement>();
        for (const [, card] of this.renderedCards) {
          const fid = this.getCardFileId(card);
          if (fid !== null) cardsByFileId.set(fid, card);
        }

        for (const [fileIdStr, dataUri] of Object.entries(thumbs)) {
          if (!dataUri) continue;
          const fileId = Number(fileIdStr);
          this.thumbCache.set(fileId, dataUri);
          if (this.thumbCache.size > THUMB_CACHE_MAX) {
            const firstKey = this.thumbCache.keys().next().value;
            if (firstKey !== undefined) this.thumbCache.delete(firstKey);
          }
          const card = cardsByFileId.get(fileId);
          if (card) {
            const thumb = card.querySelector(".file-card-thumb");
            if (thumb && thumb.isConnected && !thumb.querySelector("img")) {
              const img = document.createElement("img");
              img.src = dataUri;
              img.className = "file-card-thumb-img";
              thumb.textContent = "";
              thumb.appendChild(img);
            }
          }
        }
      }).catch(() => { /* ignore */ });
    }

    // Batch-load attachment counts for newly rendered cards
    if (newFileIds.length > 0) {
      FileService.getAttachmentCounts(newFileIds).then((counts) => {
        // Build file-ID-to-card map for O(1) lookups
        const cardMap = new Map<number, HTMLElement>();
        for (const [, card] of this.renderedCards) {
          const fid = this.getCardFileId(card);
          if (fid !== null) cardMap.set(fid, card);
        }

        for (const [fileIdStr, count] of Object.entries(counts)) {
          if (count > 0) {
            const fileId = Number(fileIdStr);
            const card = cardMap.get(fileId);
            if (card) {
              const nameEl = card.querySelector(".file-card-name");
              if (nameEl && card.isConnected && !card.querySelector(".file-card-attachment")) {
                const clip = document.createElement("span");
                clip.className = "file-card-attachment";
                clip.textContent = "\uD83D\uDCCE";
                clip.title = `${count} Anhang/Anh\u00E4nge`;
                nameEl.appendChild(clip);
              }
            }
          }
        }
      }).catch(() => { /* ignore */ });
    }
  }

  private getCardFileId(card: HTMLElement): number | null {
    const id = card.dataset.fileId;
    return id ? Number(id) : null;
  }

  private createCard(
    file: { id: number; name: string | null; filename: string; fileSizeBytes: number | null; aiAnalyzed: boolean; aiConfirmed: boolean; fileType: string },
    index: number,
    selectedId: number | null,
    selectedIds: readonly number[],
  ): HTMLElement {
    const card = document.createElement("div");
    card.className = "file-card";
    card.setAttribute("role", "listitem");
    card.setAttribute("aria-label", file.name || file.filename);
    card.dataset.fileId = String(file.id);
    card.style.position = "absolute";
    card.style.top = `${index * CARD_HEIGHT}px`;
    card.style.left = "0";
    card.style.right = "0";
    card.style.height = `${CARD_HEIGHT}px`;
    card.style.boxSizing = "border-box";

    const isMultiSelected = selectedIds.includes(file.id);
    const isSingleSelected = file.id === selectedId && selectedIds.length === 0;
    if (isMultiSelected || isSingleSelected) {
      card.classList.add("selected");
    }

    const thumb = document.createElement("div");
    thumb.className = "file-card-thumb";
    thumb.textContent = getFormatLabel(file.filename);
    const cachedUri = this.thumbCache.get(file.id);
    if (cachedUri) {
      const img = document.createElement("img");
      img.src = cachedUri;
      img.alt = file.name || file.filename;
      img.className = "file-card-thumb-img";
      thumb.textContent = "";
      thumb.appendChild(img);
    }
    // Thumbnails for uncached cards are loaded in batch by renderVisible()
    card.appendChild(thumb);

    const info = document.createElement("div");
    info.className = "file-card-info";

    const nameEl = document.createElement("div");
    nameEl.className = "file-card-name";
    nameEl.textContent = file.name || file.filename;
    info.appendChild(nameEl);

    // File type badge (S7-04)
    if (file.fileType && file.fileType !== "embroidery") {
      const typeBadge = document.createElement("span");
      const typeLabels: Record<string, string> = {
        sewing_pattern: "Schnitt",
        document: "Dok",
        reference_image: "Bild",
      };
      typeBadge.className = `file-type-badge type-${file.fileType}`;
      typeBadge.textContent = typeLabels[file.fileType] || file.fileType;
      nameEl.appendChild(typeBadge);
    }

    if (file.aiAnalyzed) {
      const badge = document.createElement("span");
      if (file.aiConfirmed) {
        badge.className = "ai-badge ai-badge--confirmed";
        badge.textContent = "KI";
        badge.title = "KI-analysiert und best\u00E4tigt";
      } else {
        badge.className = "ai-badge ai-badge--pending";
        badge.textContent = "KI";
        badge.title = "KI-analysiert, nicht best\u00E4tigt";
      }
      nameEl.appendChild(badge);
    }

    const meta = document.createElement("div");
    meta.className = "file-card-meta";
    const parts: string[] = [];
    if (file.fileSizeBytes) {
      parts.push(formatSize(file.fileSizeBytes));
    }
    const ext = getFormatLabel(file.filename);
    if (ext) {
      parts.push(ext);
    }
    meta.textContent = parts.join(" \u00B7 ");
    info.appendChild(meta);

    card.appendChild(info);

    card.addEventListener("click", (e) => {
      this.handleClick(file.id, index, e);
    });

    return card;
  }

  private updateSelection(): void {
    const files = appState.getRef("files");
    const selectedId = appState.getRef("selectedFileId");
    const selectedIds = appState.getRef("selectedFileIds");

    for (const [index, card] of this.renderedCards) {
      const file = files[index];
      if (!file) continue;

      const isMultiSelected = selectedIds.includes(file.id);
      const isSingleSelected = file.id === selectedId && selectedIds.length === 0;
      card.classList.toggle("selected", isMultiSelected || isSingleSelected);
    }
  }

  private scrollToIndex(index: number): void {
    if (!this.scrollContainer) return;
    const containerHeight = this.scrollContainer.clientHeight;
    const itemTop = index * CARD_HEIGHT;
    const itemBottom = itemTop + CARD_HEIGHT;
    const scrollTop = this.scrollContainer.scrollTop;

    if (itemTop < scrollTop) {
      this.scrollContainer.scrollTop = itemTop;
    } else if (itemBottom > scrollTop + containerHeight) {
      this.scrollContainer.scrollTop = itemBottom - containerHeight;
    }
  }

  private handleClick(fileId: number, index: number, e: MouseEvent): void {
    const files = appState.getRef("files");

    if (e.shiftKey && this.lastClickedIndex !== null) {
      // Shift+click: range select
      const start = Math.min(this.lastClickedIndex, index);
      const end = Math.max(this.lastClickedIndex, index);
      const rangeIds = files.slice(start, end + 1).map((f) => f.id);
      appState.set("selectedFileIds", rangeIds);
      appState.set("selectedFileId", fileId);
      this.lastClickedIndex = index;
    } else if (e.metaKey || e.ctrlKey) {
      // Cmd/Ctrl+click: toggle in multi-select
      const current = appState.get("selectedFileIds");
      const singleId = appState.get("selectedFileId");
      let newIds: number[];

      if (current.length === 0 && singleId !== null) {
        // Transition from single-select to multi-select
        newIds = singleId === fileId ? [] : [singleId, fileId];
      } else if (current.includes(fileId)) {
        newIds = current.filter((id) => id !== fileId);
      } else {
        newIds = [...current, fileId];
      }

      appState.set("selectedFileIds", newIds);
      appState.set("selectedFileId", newIds.length > 0 ? newIds[newIds.length - 1] : null);
      this.lastClickedIndex = index;
    } else {
      // Normal click: single select, clear multi-select
      appState.set("selectedFileIds", []);
      appState.set("selectedFileId", fileId);
      this.lastClickedIndex = index;
    }
  }

}

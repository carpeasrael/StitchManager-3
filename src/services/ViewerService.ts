import { invoke } from "@tauri-apps/api/core";
import type { InstructionBookmark, InstructionNote } from "../types";

export async function readFileBase64(filePath: string): Promise<string> {
  return invoke("read_file_bytes", { filePath });
}

export async function readFileBytes(filePath: string): Promise<Uint8Array> {
  const base64 = await readFileBase64(filePath);
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

export async function toggleBookmark(
  fileId: number,
  pageNumber: number,
  label?: string
): Promise<boolean> {
  return invoke("toggle_bookmark", { fileId, pageNumber, label: label ?? null });
}

export async function getBookmarks(
  fileId: number
): Promise<InstructionBookmark[]> {
  return invoke("get_bookmarks", { fileId });
}

export async function updateBookmarkLabel(
  bookmarkId: number,
  label: string
): Promise<void> {
  return invoke("update_bookmark_label", { bookmarkId, label });
}

export async function addNote(
  fileId: number,
  pageNumber: number,
  noteText: string
): Promise<InstructionNote> {
  return invoke("add_note", { fileId, pageNumber, noteText });
}

export async function updateNote(
  noteId: number,
  noteText: string
): Promise<void> {
  return invoke("update_note", { noteId, noteText });
}

export async function deleteNote(noteId: number): Promise<void> {
  return invoke("delete_note", { noteId });
}

export async function getNotes(
  fileId: number,
  pageNumber?: number
): Promise<InstructionNote[]> {
  return invoke("get_notes", { fileId, pageNumber: pageNumber ?? null });
}

export async function setLastViewedPage(
  fileId: number,
  pageNumber: number
): Promise<void> {
  return invoke("set_last_viewed_page", { fileId, pageNumber });
}

export async function getLastViewedPage(
  fileId: number
): Promise<number | null> {
  return invoke("get_last_viewed_page", { fileId });
}

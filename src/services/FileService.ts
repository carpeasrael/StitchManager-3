import { invoke } from "@tauri-apps/api/core";
import type {
  EmbroideryFile,
  FileFormat,
  ThreadColor,
  Tag,
} from "../types/index";

export async function getFiles(
  folderId?: number | null,
  search?: string | null,
  formatFilter?: string | null
): Promise<EmbroideryFile[]> {
  return invoke<EmbroideryFile[]>("get_files", {
    folderId: folderId ?? null,
    search: search ?? null,
    formatFilter: formatFilter ?? null,
  });
}

export async function getFile(fileId: number): Promise<EmbroideryFile> {
  return invoke<EmbroideryFile>("get_file", { fileId });
}

export async function getFormats(fileId: number): Promise<FileFormat[]> {
  return invoke<FileFormat[]>("get_file_formats", { fileId });
}

export async function getColors(fileId: number): Promise<ThreadColor[]> {
  return invoke<ThreadColor[]>("get_file_colors", { fileId });
}

export async function getTags(fileId: number): Promise<Tag[]> {
  return invoke<Tag[]>("get_file_tags", { fileId });
}

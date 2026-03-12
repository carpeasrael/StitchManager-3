import { invoke } from "@tauri-apps/api/core";
import type {
  EmbroideryFile,
  FileFormat,
  FileUpdate,
  ThreadColor,
  Tag,
  StitchSegment,
  SearchParams,
} from "../types/index";

export async function getFiles(
  folderId?: number | null,
  search?: string | null,
  formatFilter?: string | null,
  searchParams?: SearchParams | null
): Promise<EmbroideryFile[]> {
  return invoke<EmbroideryFile[]>("get_files", {
    folderId: folderId ?? null,
    search: search ?? null,
    formatFilter: formatFilter ?? null,
    searchParams: searchParams ?? null,
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

export async function updateFile(
  fileId: number,
  updates: FileUpdate
): Promise<EmbroideryFile> {
  return invoke<EmbroideryFile>("update_file", { fileId, updates });
}

export async function deleteFile(fileId: number): Promise<void> {
  return invoke<void>("delete_file", { fileId });
}

export async function setTags(
  fileId: number,
  tagNames: string[]
): Promise<Tag[]> {
  return invoke<Tag[]>("set_file_tags", { fileId, tagNames });
}

export async function getAllTags(): Promise<Tag[]> {
  return invoke<Tag[]>("get_all_tags");
}

export async function getThumbnail(fileId: number): Promise<string> {
  return invoke<string>("get_thumbnail", { fileId });
}

export async function getStitchSegments(
  filepath: string
): Promise<StitchSegment[]> {
  return invoke<StitchSegment[]>("get_stitch_segments", { filepath });
}

import { invoke } from "@tauri-apps/api/core";
import type {
  EmbroideryFile,
  FileAttachment,
  FileFormat,
  FileUpdate,
  ThreadColor,
  Tag,
  StitchSegment,
  SearchParams,
} from "../types/index";

export interface PaginatedFiles {
  files: EmbroideryFile[];
  totalCount: number;
  page: number;
  pageSize: number;
}

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

export async function getFilesPaginated(
  folderId?: number | null,
  search?: string | null,
  formatFilter?: string | null,
  searchParams?: SearchParams | null,
  page?: number,
  pageSize?: number
): Promise<PaginatedFiles> {
  return invoke<PaginatedFiles>("get_files_paginated", {
    folderId: folderId ?? null,
    search: search ?? null,
    formatFilter: formatFilter ?? null,
    searchParams: searchParams ?? null,
    page: page ?? 0,
    pageSize: pageSize ?? 200,
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

export async function getThumbnailsBatch(
  fileIds: number[]
): Promise<Record<number, string>> {
  return invoke<Record<number, string>>("get_thumbnails_batch", { fileIds });
}

export async function getStitchSegments(
  filepath: string
): Promise<StitchSegment[]> {
  return invoke<StitchSegment[]>("get_stitch_segments", { filepath });
}

export async function getAttachments(
  fileId: number
): Promise<FileAttachment[]> {
  return invoke<FileAttachment[]>("get_attachments", { fileId });
}

export async function attachFile(
  fileId: number,
  sourcePath: string,
  attachmentType: string
): Promise<FileAttachment> {
  return invoke<FileAttachment>("attach_file", {
    fileId,
    sourcePath,
    attachmentType,
  });
}

export async function deleteAttachment(
  attachmentId: number
): Promise<void> {
  return invoke<void>("delete_attachment", { attachmentId });
}

export async function openAttachment(
  attachmentId: number
): Promise<void> {
  return invoke<void>("open_attachment", { attachmentId });
}

export async function getAttachmentCount(
  fileId: number
): Promise<number> {
  return invoke<number>("get_attachment_count", { fileId });
}

export async function getAttachmentCounts(
  fileIds: number[]
): Promise<Record<number, number>> {
  return invoke<Record<number, number>>("get_attachment_counts", { fileIds });
}

export async function generatePdfReport(
  fileIds: number[]
): Promise<string> {
  return invoke<string>("generate_pdf_report", { fileIds });
}

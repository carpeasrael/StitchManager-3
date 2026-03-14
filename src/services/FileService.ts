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
  LibraryStats,
  Transform,
  FileVersion,
  MachineProfile,
  TransferResult,
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

export async function getRecentFiles(limit?: number): Promise<EmbroideryFile[]> {
  return invoke<EmbroideryFile[]>("get_recent_files", { limit: limit ?? 20 });
}

export async function getFavoriteFiles(): Promise<EmbroideryFile[]> {
  return invoke<EmbroideryFile[]>("get_favorite_files");
}

export async function toggleFavorite(fileId: number): Promise<boolean> {
  return invoke<boolean>("toggle_favorite", { fileId });
}

export async function getLibraryStats(): Promise<LibraryStats> {
  return invoke<LibraryStats>("get_library_stats");
}

export async function getSupportedFormats(): Promise<string[]> {
  return invoke<string[]>("get_supported_formats");
}

export async function convertFile(
  fileId: number,
  targetFormat: string,
  outputDir: string
): Promise<string> {
  return invoke<string>("convert_file", { fileId, targetFormat, outputDir });
}

export async function convertFilesBatch(
  fileIds: number[],
  targetFormat: string,
  outputDir: string
): Promise<{ total: number; success: number; failed: number; errors: string[] }> {
  return invoke("convert_files_batch", { fileIds, targetFormat, outputDir });
}

export async function previewTransform(
  fileId: number,
  transforms: Transform[]
): Promise<StitchSegment[]> {
  return invoke<StitchSegment[]>("preview_transform", { fileId, transforms });
}

export async function saveTransformed(
  fileId: number,
  transforms: Transform[],
  outputPath: string
): Promise<string> {
  return invoke<string>("save_transformed", { fileId, transforms, outputPath });
}

export async function getStitchDimensions(
  fileId: number
): Promise<[number, number]> {
  return invoke<[number, number]>("get_stitch_dimensions", { fileId });
}

// Version history
export async function getFileVersions(fileId: number): Promise<FileVersion[]> {
  return invoke<FileVersion[]>("get_file_versions", { fileId });
}

export async function restoreVersion(fileId: number, versionId: number): Promise<void> {
  return invoke<void>("restore_version", { fileId, versionId });
}

export async function deleteVersion(versionId: number): Promise<void> {
  return invoke<void>("delete_version", { versionId });
}

export async function exportVersion(versionId: number, path: string): Promise<void> {
  return invoke<void>("export_version", { versionId, path });
}

// Machine transfer
export async function listMachines(): Promise<MachineProfile[]> {
  return invoke<MachineProfile[]>("list_machines");
}

export async function addMachine(
  name: string, machineType: string, transferPath: string, targetFormat?: string
): Promise<MachineProfile> {
  return invoke<MachineProfile>("add_machine", { name, machineType, transferPath, targetFormat: targetFormat ?? null });
}

export async function deleteMachine(machineId: number): Promise<void> {
  return invoke<void>("delete_machine", { machineId });
}

export async function testMachineConnection(machineId: number): Promise<boolean> {
  return invoke<boolean>("test_machine_connection", { machineId });
}

export async function transferFiles(machineId: number, fileIds: number[]): Promise<TransferResult> {
  return invoke<TransferResult>("transfer_files", { machineId, fileIds });
}

export async function generatePdfReport(
  fileIds: number[]
): Promise<string> {
  return invoke<string>("generate_pdf_report", { fileIds });
}

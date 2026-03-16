import { invoke } from "@tauri-apps/api/core";

export interface BackupResult {
  path: string;
  sizeBytes: number;
  fileCount: number;
}

export async function createBackup(includeFiles: boolean): Promise<BackupResult> {
  return invoke("create_backup", { includeFiles });
}

export async function restoreBackup(backupPath: string): Promise<string> {
  return invoke("restore_backup", { backupPath });
}

export async function checkMissingFiles(): Promise<[number, string][]> {
  return invoke("check_missing_files");
}

export async function relinkFile(fileId: number, newPath: string): Promise<void> {
  return invoke("relink_file", { fileId, newPath });
}

export async function relinkBatch(oldPrefix: string, newPrefix: string): Promise<number> {
  return invoke("relink_batch", { oldPrefix, newPrefix });
}

export async function exportMetadataJson(fileIds: number[]): Promise<string> {
  return invoke("export_metadata_json", { fileIds });
}

export async function exportMetadataCsv(fileIds: number[]): Promise<string> {
  return invoke("export_metadata_csv", { fileIds });
}

export async function softDeleteFile(fileId: number): Promise<void> {
  return invoke("soft_delete_file", { fileId });
}

export async function restoreFile(fileId: number): Promise<void> {
  return invoke("restore_file", { fileId });
}

export async function getTrash(): Promise<[number, string, string][]> {
  return invoke("get_trash");
}

export async function purgeFile(fileId: number): Promise<void> {
  return invoke("purge_file", { fileId });
}

export async function autoPurgeTrash(): Promise<number> {
  return invoke("auto_purge_trash");
}

export async function archiveFile(fileId: number): Promise<void> {
  return invoke("archive_file", { fileId });
}

export async function unarchiveFile(fileId: number): Promise<void> {
  return invoke("unarchive_file", { fileId });
}

export async function importMetadataJson(jsonData: string): Promise<number> {
  return invoke("import_metadata_json", { jsonData });
}

export async function archiveFilesBatch(fileIds: number[]): Promise<number> {
  return invoke("archive_files_batch", { fileIds });
}

export async function unarchiveFilesBatch(fileIds: number[]): Promise<number> {
  return invoke("unarchive_files_batch", { fileIds });
}

export async function exportLibrary(): Promise<string> {
  return invoke("export_library");
}

export async function importLibrary(jsonPath: string, newLibraryRoot: string): Promise<number> {
  return invoke("import_library", { jsonPath, newLibraryRoot });
}

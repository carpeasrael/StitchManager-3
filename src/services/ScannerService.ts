import { invoke } from "@tauri-apps/api/core";
import type { EmbroideryFile, MassImportResult, MigrationResult, ScanOnlyResult, BulkImportMetadata } from "../types/index";

export interface ScanResult {
  foundFiles: string[];
  totalScanned: number;
  errors: string[];
}

export async function scanDirectory(path: string): Promise<ScanResult> {
  return invoke<ScanResult>("scan_directory", { path });
}

export async function scanOnly(path: string): Promise<ScanOnlyResult> {
  return invoke<ScanOnlyResult>("scan_only", { path });
}

export async function importFiles(
  filePaths: string[],
  folderId: number,
  bulkMetadata?: BulkImportMetadata
): Promise<EmbroideryFile[]> {
  return invoke<EmbroideryFile[]>("import_files", {
    filePaths,
    folderId,
    bulkMetadata: bulkMetadata ?? null,
  });
}

export async function massImport(path: string): Promise<MassImportResult> {
  return invoke<MassImportResult>("mass_import", { path });
}

export async function migrateFrom2Stitch(xmlPath?: string): Promise<MigrationResult> {
  return invoke<MigrationResult>("migrate_from_2stitch", { xmlPath: xmlPath ?? null });
}

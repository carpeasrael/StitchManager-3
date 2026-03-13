import { invoke } from "@tauri-apps/api/core";
import type { EmbroideryFile, MassImportResult } from "../types/index";

export interface ScanResult {
  foundFiles: string[];
  totalScanned: number;
  errors: string[];
}

export async function scanDirectory(path: string): Promise<ScanResult> {
  return invoke<ScanResult>("scan_directory", { path });
}

export async function importFiles(
  filePaths: string[],
  folderId: number
): Promise<EmbroideryFile[]> {
  return invoke<EmbroideryFile[]>("import_files", { filePaths, folderId });
}

export async function massImport(path: string): Promise<MassImportResult> {
  return invoke<MassImportResult>("mass_import", { path });
}

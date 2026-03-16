import { invoke } from "@tauri-apps/api/core";
import type { PrinterInfo, PrintSettings, TileInfo } from "../types";

export async function getPrinters(): Promise<PrinterInfo[]> {
  return invoke("get_printers");
}

export async function printPdf(
  filePath: string,
  settings: PrintSettings
): Promise<void> {
  return invoke("print_pdf", { filePath, settings });
}

export async function savePrintSettings(
  paperSize: string,
  orientation: string,
  printerName: string | null
): Promise<void> {
  return invoke("save_print_settings", { paperSize, orientation, printerName });
}

export async function loadPrintSettings(): Promise<Record<string, string>> {
  return invoke("load_print_settings");
}

export async function computeTiles(
  pageWidthMm: number,
  pageHeightMm: number,
  paperSize: string,
  overlapMm: number
): Promise<TileInfo> {
  return invoke("compute_tiles", { pageWidthMm, pageHeightMm, paperSize, overlapMm });
}

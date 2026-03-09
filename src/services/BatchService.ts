import { invoke } from "@tauri-apps/api/core";
import type { BatchResult } from "../types/index";

export async function rename(
  fileIds: number[],
  pattern: string
): Promise<BatchResult> {
  return invoke<BatchResult>("batch_rename", { fileIds, pattern });
}

export async function organize(
  fileIds: number[],
  pattern: string
): Promise<BatchResult> {
  return invoke<BatchResult>("batch_organize", { fileIds, pattern });
}

export async function exportUsb(
  fileIds: number[],
  targetPath: string
): Promise<BatchResult> {
  return invoke<BatchResult>("batch_export_usb", { fileIds, targetPath });
}

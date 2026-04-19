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

/**
 * Audit Wave 5 (deferred from Wave 3 #4): cooperatively cancel the
 * currently running batch operation. The backend checks the cancel flag
 * once per file and aborts the loop cleanly (returning whatever
 * progress was made so far).
 */
export async function cancelBatch(): Promise<void> {
  return invoke<void>("cancel_batch");
}

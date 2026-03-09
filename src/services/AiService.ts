import { invoke } from "@tauri-apps/api/core";
import type {
  AiAnalysisResult,
  EmbroideryFile,
  SelectedFields,
} from "../types/index";

export async function buildPrompt(fileId: number): Promise<string> {
  return invoke<string>("ai_build_prompt", { fileId });
}

export async function analyzeFile(
  fileId: number,
  prompt: string
): Promise<AiAnalysisResult> {
  return invoke<AiAnalysisResult>("ai_analyze_file", { fileId, prompt });
}

export async function acceptResult(
  resultId: number,
  selectedFields: SelectedFields
): Promise<EmbroideryFile> {
  return invoke<EmbroideryFile>("ai_accept_result", {
    resultId,
    selectedFields,
  });
}

export async function rejectResult(resultId: number): Promise<void> {
  return invoke<void>("ai_reject_result", { resultId });
}

export async function testConnection(): Promise<boolean> {
  return invoke<boolean>("ai_test_connection");
}

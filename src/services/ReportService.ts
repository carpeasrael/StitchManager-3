import { invoke } from "@tauri-apps/api/core";
import type { ProjectReport } from "../types/index";

export async function getProjectReport(
  projectId: number,
  laborRate?: number
): Promise<ProjectReport> {
  return invoke("get_project_report", { projectId, laborRate });
}

export async function exportProjectCsv(
  projectId: number,
  laborRate?: number
): Promise<string> {
  return invoke("export_project_csv", { projectId, laborRate });
}

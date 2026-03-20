import { invoke } from "@tauri-apps/api/core";
import type { DashboardStats } from "../types/index";

export async function getDashboardStats(): Promise<DashboardStats> {
  return invoke<DashboardStats>("get_dashboard_stats");
}

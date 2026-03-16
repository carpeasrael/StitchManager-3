import { invoke } from "@tauri-apps/api/core";
import type { CostBreakdown, CostRate, ProjectReport } from "../types/index";

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

// Cost Rate CRUD

export async function listCostRates(
  rateType?: string
): Promise<CostRate[]> {
  return invoke("list_cost_rates", { rateType });
}

export async function createCostRate(
  rateType: string,
  name: string,
  rateValue: number,
  unit?: string,
  setupCost?: number,
  notes?: string
): Promise<CostRate> {
  return invoke("create_cost_rate", { rateType, name, rateValue, unit, setupCost, notes });
}

export async function updateCostRate(
  rateId: number,
  name?: string,
  rateValue?: number,
  unit?: string,
  setupCost?: number,
  notes?: string
): Promise<CostRate> {
  return invoke("update_cost_rate", { rateId, name, rateValue, unit, setupCost, notes });
}

export async function deleteCostRate(rateId: number): Promise<void> {
  return invoke("delete_cost_rate", { rateId });
}

// Cost Breakdown

export async function getCostBreakdown(
  projectId: number
): Promise<CostBreakdown> {
  return invoke("get_cost_breakdown", { projectId });
}

export async function calculateSellingPrice(
  projectId: number,
  overrideProfitPct?: number
): Promise<CostBreakdown> {
  return invoke("calculate_selling_price", { projectId, overrideProfitPct });
}

export async function saveCostBreakdown(
  projectId: number
): Promise<CostBreakdown> {
  return invoke("save_cost_breakdown", { projectId });
}

// Project-License Links

export async function linkLicenseToProject(
  projectId: number,
  licenseId: number
): Promise<void> {
  return invoke("link_license_to_project", { projectId, licenseId });
}

export async function unlinkLicenseFromProject(
  projectId: number,
  licenseId: number
): Promise<void> {
  return invoke("unlink_license_from_project", { projectId, licenseId });
}

export async function getProjectLicenses(
  projectId: number
): Promise<import("../types/index").LicenseRecord[]> {
  return invoke("get_project_licenses", { projectId });
}

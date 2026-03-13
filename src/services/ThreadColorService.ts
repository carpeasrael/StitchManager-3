import { invoke } from "@tauri-apps/api/core";
import type { ThreadMatch, BrandColor } from "../types/index";

export async function getThreadMatches(
  colorHex: string,
  brands?: string[],
  limit?: number
): Promise<ThreadMatch[]> {
  return invoke<ThreadMatch[]>("get_thread_matches", {
    colorHex,
    brands: brands ?? null,
    limit: limit ?? null,
  });
}

export async function getAvailableBrands(): Promise<string[]> {
  return invoke<string[]>("get_available_brands");
}

export async function getBrandColors(brand: string): Promise<BrandColor[]> {
  return invoke<BrandColor[]>("get_brand_colors", { brand });
}

import { invoke } from "@tauri-apps/api/core";
import type { SmartFolder } from "../types/index";

export async function getAll(): Promise<SmartFolder[]> {
  return invoke<SmartFolder[]>("get_smart_folders");
}

export async function create(
  name: string,
  filterJson: string,
  icon?: string
): Promise<SmartFolder> {
  return invoke<SmartFolder>("create_smart_folder", {
    name,
    icon: icon ?? null,
    filterJson,
  });
}

export async function update(
  id: number,
  name?: string,
  icon?: string,
  filterJson?: string
): Promise<SmartFolder> {
  return invoke<SmartFolder>("update_smart_folder", {
    id,
    name: name ?? null,
    icon: icon ?? null,
    filterJson: filterJson ?? null,
  });
}

export async function remove(id: number): Promise<void> {
  return invoke<void>("delete_smart_folder", { id });
}

export async function updateSortOrders(
  orders: [number, number][]
): Promise<void> {
  return invoke<void>("update_smart_folder_sort_orders", { orders });
}

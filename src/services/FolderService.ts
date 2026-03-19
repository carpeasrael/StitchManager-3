import { invoke } from "@tauri-apps/api/core";
import type { Folder, FolderType } from "../types/index";

export async function getAll(): Promise<Folder[]> {
  return invoke<Folder[]>("get_folders");
}

export async function create(
  name: string,
  path: string,
  parentId?: number | null,
  folderType?: FolderType
): Promise<Folder> {
  return invoke<Folder>("create_folder", {
    name,
    path,
    parentId: parentId ?? null,
    folderType: folderType ?? null,
  });
}

export async function update(
  folderId: number,
  name?: string,
  folderType?: FolderType
): Promise<Folder> {
  return invoke<Folder>("update_folder", {
    folderId,
    name: name ?? null,
    folderType: folderType ?? null,
  });
}

export async function remove(folderId: number): Promise<void> {
  return invoke<void>("delete_folder", { folderId });
}

export async function updateSortOrders(
  folderOrders: [number, number][]
): Promise<void> {
  return invoke<void>("update_folder_sort_orders", { folderOrders });
}

export async function getFileCount(folderId: number): Promise<number> {
  return invoke<number>("get_folder_file_count", { folderId });
}

export async function getAllFileCounts(): Promise<Record<number, number>> {
  return invoke<Record<number, number>>("get_all_folder_file_counts");
}

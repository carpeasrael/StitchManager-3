import { invoke } from "@tauri-apps/api/core";
import type { Project, ProjectDetail, Collection } from "../types";

// --- Projects ---

export async function createProject(project: {
  name: string;
  patternFileId?: number | null;
  status?: string;
  notes?: string;
  orderNumber?: string;
  customer?: string;
  priority?: string;
  deadline?: string;
  responsiblePerson?: string;
  approvalStatus?: string;
}): Promise<Project> {
  return invoke("create_project", { project });
}

export async function getProjects(
  statusFilter?: string,
  patternFileId?: number
): Promise<Project[]> {
  return invoke("get_projects", {
    statusFilter: statusFilter ?? null,
    patternFileId: patternFileId ?? null,
  });
}

export async function getProject(projectId: number): Promise<Project> {
  return invoke("get_project", { projectId });
}

export async function updateProject(
  projectId: number,
  update: {
    name?: string;
    status?: string;
    notes?: string;
    orderNumber?: string;
    customer?: string;
    priority?: string;
    deadline?: string;
    responsiblePerson?: string;
    approvalStatus?: string;
  }
): Promise<Project> {
  return invoke("update_project", { projectId, update });
}

export async function deleteProject(projectId: number): Promise<void> {
  return invoke("delete_project", { projectId });
}

export async function duplicateProject(
  projectId: number,
  newName?: string
): Promise<Project> {
  return invoke("duplicate_project", { projectId, newName: newName ?? null });
}

export async function setProjectDetails(
  projectId: number,
  details: { key: string; value: string | null }[]
): Promise<void> {
  return invoke("set_project_details", { projectId, details });
}

export async function getProjectDetails(
  projectId: number
): Promise<ProjectDetail[]> {
  return invoke("get_project_details", { projectId });
}

// --- Collections ---

export async function createCollection(
  name: string,
  description?: string
): Promise<Collection> {
  return invoke("create_collection", { name, description: description ?? null });
}

export async function getCollections(): Promise<Collection[]> {
  return invoke("get_collections");
}

export async function deleteCollection(collectionId: number): Promise<void> {
  return invoke("delete_collection", { collectionId });
}

export async function addToCollection(
  collectionId: number,
  fileId: number
): Promise<void> {
  return invoke("add_to_collection", { collectionId, fileId });
}

export async function removeFromCollection(
  collectionId: number,
  fileId: number
): Promise<void> {
  return invoke("remove_from_collection", { collectionId, fileId });
}

export async function getCollectionFiles(
  collectionId: number
): Promise<number[]> {
  return invoke("get_collection_files", { collectionId });
}

// --- Project Products ---

export interface ProjectProduct {
  id: number;
  projectId: number;
  productId: number;
  productName: string;
  quantity: number;
  sortOrder: number;
}

export async function linkProductToProject(
  projectId: number,
  productId: number,
  quantity?: number
): Promise<ProjectProduct> {
  return invoke("link_product_to_project", {
    projectId,
    productId,
    quantity: quantity ?? null,
  });
}

export async function unlinkProductFromProject(
  projectId: number,
  productId: number
): Promise<void> {
  return invoke("unlink_product_from_project", { projectId, productId });
}

export async function getProjectProducts(
  projectId: number
): Promise<ProjectProduct[]> {
  return invoke("get_project_products", { projectId });
}

// --- Project Files ---

export interface ProjectFile {
  id: number;
  projectId: number;
  fileId: number;
  filename: string;
  role: string;
  sortOrder: number;
}

export async function addFileToProject(
  projectId: number,
  fileId: number,
  role: string
): Promise<ProjectFile> {
  return invoke("add_file_to_project", { projectId, fileId, role });
}

export async function removeFileFromProject(
  projectId: number,
  fileId: number,
  role: string
): Promise<void> {
  return invoke("remove_file_from_project", { projectId, fileId, role });
}

export async function getProjectFiles(
  projectId: number
): Promise<ProjectFile[]> {
  return invoke("get_project_files", { projectId });
}

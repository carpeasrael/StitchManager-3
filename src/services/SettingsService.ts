import { invoke } from "@tauri-apps/api/core";
import type { CustomFieldDef } from "../types/index";

export async function getSetting(key: string): Promise<string> {
  return invoke<string>("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}

export async function getAllSettings(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("get_all_settings");
}

export async function getCustomFields(): Promise<CustomFieldDef[]> {
  return invoke<CustomFieldDef[]>("get_custom_fields");
}

export async function createCustomField(
  name: string,
  fieldType: string,
  options?: string
): Promise<CustomFieldDef> {
  return invoke<CustomFieldDef>("create_custom_field", {
    name,
    fieldType,
    options: options ?? null,
  });
}

export async function deleteCustomField(fieldId: number): Promise<void> {
  return invoke<void>("delete_custom_field", { fieldId });
}

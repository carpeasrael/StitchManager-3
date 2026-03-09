export interface Folder {
  id: number;
  name: string;
  path: string;
  parentId: number | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface EmbroideryFile {
  id: number;
  folderId: number;
  filename: string;
  filepath: string;
  name: string | null;
  theme: string | null;
  description: string | null;
  license: string | null;
  widthMm: number | null;
  heightMm: number | null;
  stitchCount: number | null;
  colorCount: number | null;
  fileSizeBytes: number | null;
  thumbnailPath: string | null;
  aiAnalyzed: boolean;
  aiConfirmed: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface FileFormat {
  id: number;
  fileId: number;
  format: string;
  formatVersion: string | null;
  filepath: string;
  fileSizeBytes: number | null;
  parsed: boolean;
}

export interface ThreadColor {
  id: number;
  fileId: number;
  sortOrder: number;
  colorHex: string;
  colorName: string | null;
  brand: string | null;
  brandCode: string | null;
  isAi: boolean;
}

export interface Tag {
  id: number;
  name: string;
  createdAt: string;
}

export interface AiAnalysisResult {
  id: number;
  fileId: number;
  provider: string;
  model: string;
  promptHash: string | null;
  rawResponse: string | null;
  parsedName: string | null;
  parsedTheme: string | null;
  parsedDesc: string | null;
  parsedTags: string | null;
  parsedColors: string | null;
  accepted: boolean;
  analyzedAt: string;
}

export interface FileUpdate {
  name?: string;
  theme?: string;
  description?: string;
  license?: string;
}

export interface CustomFieldDef {
  id: number;
  name: string;
  fieldType: string;
  options: string | null;
  required: boolean;
  sortOrder: number;
  createdAt: string;
}

export interface SelectedFields {
  name?: boolean;
  theme?: boolean;
  description?: boolean;
  tags?: boolean;
  colors?: boolean;
}

export type ThemeMode = "hell" | "dunkel";

export interface BatchResult {
  total: number;
  success: number;
  failed: number;
  errors: string[];
}

export interface State {
  folders: Folder[];
  selectedFolderId: number | null;
  files: EmbroideryFile[];
  selectedFileId: number | null;
  selectedFileIds: number[];
  searchQuery: string;
  formatFilter: string | null;
  settings: Record<string, string>;
  theme: ThemeMode;
}

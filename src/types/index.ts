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
  designName: string | null;
  jumpCount: number | null;
  trimCount: number | null;
  hoopWidthMm: number | null;
  hoopHeightMm: number | null;
  category: string | null;
  author: string | null;
  keywords: string | null;
  comments: string | null;
  uniqueId: string | null;
  isFavorite: boolean;
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

export interface StitchSegment {
  colorIndex: number;
  colorHex: string | null;
  points: [number, number][];
}

export interface FileAttachment {
  id: number;
  fileId: number;
  filename: string;
  mimeType: string | null;
  filePath: string;
  attachmentType: string;
  createdAt: string;
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

export interface SearchParams {
  text?: string;
  tags?: string[];
  stitchCountMin?: number;
  stitchCountMax?: number;
  colorCountMin?: number;
  colorCountMax?: number;
  widthMmMin?: number;
  widthMmMax?: number;
  heightMmMin?: number;
  heightMmMax?: number;
  fileSizeMin?: number;
  fileSizeMax?: number;
  aiAnalyzed?: boolean;
  aiConfirmed?: boolean;
  colorSearch?: string;
}

export interface SelectedFields {
  name?: boolean;
  theme?: boolean;
  description?: boolean;
  tags?: boolean;
  colors?: boolean;
}

export interface ImportProgress {
  current: number;
  total: number;
  filename: string;
  status: string;
  elapsedMs: number;
  estimatedRemainingMs: number;
}

export interface MassImportResult {
  folderId: number;
  importedCount: number;
  skippedCount: number;
  errorCount: number;
  elapsedMs: number;
}

export interface MigrationResult {
  foldersCreated: number;
  filesImported: number;
  filesSkipped: number;
  tagsImported: number;
  elapsedMs: number;
}

export interface UsbDevice {
  name: string;
  mountPoint: string;
  totalSpaceBytes: number;
  freeSpaceBytes: number;
}

export interface ThreadMatch {
  brand: string;
  code: string;
  name: string;
  hex: string;
  deltaE: number;
}

export interface BrandColor {
  brand: string;
  code: string;
  name: string;
  hex: string;
}

export interface LibraryStats {
  totalFiles: number;
  totalFolders: number;
  totalStitches: number;
  formatCounts: Record<string, number>;
}

export type Transform =
  | { type: "resize"; scaleX: number; scaleY: number }
  | { type: "rotate"; degrees: number }
  | { type: "mirrorHorizontal" }
  | { type: "mirrorVertical" };

export interface FileVersion {
  id: number;
  fileId: number;
  versionNumber: number;
  fileSize: number;
  operation: string;
  description: string | null;
  createdAt: string;
}

export interface MachineProfile {
  id: number;
  name: string;
  machineType: string;
  transferPath: string;
  targetFormat: string | null;
  lastUsed: string | null;
  createdAt: string;
}

export interface TransferResult {
  total: number;
  success: number;
  failed: number;
  errors: string[];
}

export type ThemeMode = "hell" | "dunkel";

export interface BatchResult {
  total: number;
  success: number;
  failed: number;
  errors: string[];
}

export type ToastLevel = "success" | "error" | "info";

export interface Toast {
  id: string;
  level: ToastLevel;
  message: string;
}

export interface State {
  folders: Folder[];
  selectedFolderId: number | null;
  files: EmbroideryFile[];
  selectedFileId: number | null;
  selectedFileIds: number[];
  searchQuery: string;
  searchParams: SearchParams;
  formatFilter: string | null;
  settings: Record<string, string>;
  theme: ThemeMode;
  toasts: Toast[];
  usbDevices: UsbDevice[];
}

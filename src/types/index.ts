export type FolderType = 'embroidery' | 'sewing_pattern' | 'mixed';

export interface Folder {
  id: number;
  name: string;
  path: string;
  parentId: number | null;
  sortOrder: number;
  folderType: FolderType;
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
  fileType: string;
  sizeRange: string | null;
  skillLevel: string | null;
  language: string | null;
  formatType: string | null;
  fileSource: string | null;
  purchaseLink: string | null;
  status: string;
  pageCount: number | null;
  paperSize: string | null;
  aiAnalyzed: boolean;
  aiConfirmed: boolean;
  createdAt: string;
  updatedAt: string;
  instructionsHtml: string | null;
  patternDate: string | null;
  rating: number | null;
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
  sizeRange?: string;
  skillLevel?: string;
  language?: string;
  formatType?: string;
  fileSource?: string;
  purchaseLink?: string;
  status?: string;
  author?: string;
  instructionsHtml?: string;
  patternDate?: string;
  rating?: number;
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
  displayName: string | null;
  sortOrder: number;
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
  fileType?: string;
  status?: string;
  skillLevel?: string;
  language?: string;
  fileSource?: string;
  category?: string;
  author?: string;
  sizeRange?: string;
  sortField?: string;
  sortDirection?: string;
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

export interface ScannedFileInfo {
  filepath: string;
  filename: string;
  fileSize: number | null;
  extension: string | null;
  fileType: string;
  alreadyImported: boolean;
}

export interface ScanOnlyResult {
  files: ScannedFileInfo[];
  totalScanned: number;
  errors: string[];
}

export interface BulkImportMetadata {
  tags?: string[];
  rating?: number;
  theme?: string;
  author?: string;
  skillLevel?: string;
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

export interface InstructionBookmark {
  id: number;
  fileId: number;
  pageNumber: number;
  label: string | null;
  createdAt: string;
}

export interface InstructionNote {
  id: number;
  fileId: number;
  pageNumber: number;
  noteText: string;
  createdAt: string;
  updatedAt: string;
}

export interface ViewerOpenEvent {
  filePath: string;
  fileId: number;
  fileName: string;
}

export interface Project {
  id: number;
  name: string;
  patternFileId: number | null;
  status: string;
  notes: string | null;
  orderNumber: string | null;
  customer: string | null;
  priority: string | null;
  deadline: string | null;
  responsiblePerson: string | null;
  approvalStatus: string | null;
  quantity: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectDetail {
  id: number;
  projectId: number;
  key: string;
  value: string | null;
}

export interface Collection {
  id: number;
  name: string;
  description: string | null;
  createdAt: string;
}

export interface Supplier {
  id: number;
  name: string;
  contact: string | null;
  website: string | null;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Material {
  id: number;
  materialNumber: string | null;
  name: string;
  materialType: string | null;
  unit: string | null;
  supplierId: number | null;
  netPrice: number | null;
  wasteFactor: number | null;
  minStock: number | null;
  reorderTimeDays: number | null;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface MaterialInventory {
  id: number;
  materialId: number;
  totalStock: number;
  reservedStock: number;
  location: string | null;
  updatedAt: string;
}

export interface MaterialConsumption {
  id: number;
  projectId: number;
  materialId: number;
  quantity: number;
  unit: string | null;
  stepName: string | null;
  recordedBy: string | null;
  notes: string | null;
  recordedAt: string;
}

export interface NachkalkulationLine {
  materialId: number;
  materialName: string;
  unit: string | null;
  plannedQuantity: number;
  actualQuantity: number;
  difference: number;
  plannedCost: number;
  actualCost: number;
  costDifference: number;
}

export interface Product {
  id: number;
  productNumber: string | null;
  name: string;
  category: string | null;
  description: string | null;
  productType: string | null;
  status: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ProductVariant {
  id: number;
  productId: number;
  sku: string | null;
  variantName: string | null;
  size: string | null;
  color: string | null;
  additionalCost: number;
  description: string | null;
  notes: string | null;
  status: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface BillOfMaterial {
  id: number;
  productId: number;
  entryType: string;
  materialId: number | null;
  stepDefinitionId: number | null;
  fileId: number | null;
  quantity: number;
  unit: string | null;
  durationMinutes: number | null;
  label: string | null;
  notes: string | null;
  sortOrder: number;
}

export interface TimeEntry {
  id: number;
  projectId: number;
  stepName: string;
  plannedMinutes: number | null;
  actualMinutes: number | null;
  worker: string | null;
  machine: string | null;
  costRateId: number | null;
  recordedAt: string;
}

// Sprint D: Production Workflow

export interface StepDefinition {
  id: number;
  name: string;
  description: string | null;
  defaultDurationMinutes: number | null;
  sortOrder: number;
  createdAt: string;
}

export interface ProductStep {
  id: number;
  productId: number;
  stepDefinitionId: number;
  sortOrder: number;
}

export interface WorkflowStep {
  id: number;
  projectId: number;
  stepDefinitionId: number;
  status: string;
  responsible: string | null;
  startedAt: string | null;
  completedAt: string | null;
  notes: string | null;
  sortOrder: number;
}

// Sprint E: Procurement

export interface PurchaseOrder {
  id: number;
  orderNumber: string | null;
  supplierId: number;
  projectId: number | null;
  status: string;
  orderDate: string | null;
  expectedDelivery: string | null;
  shippingCost: number;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface MaterialRequirement {
  materialId: number;
  materialName: string;
  unit: string | null;
  needed: number;
  available: number;
  shortage: number;
  supplierId: number | null;
  supplierName: string | null;
}

export interface OrderItem {
  id: number;
  orderId: number;
  materialId: number;
  quantityOrdered: number;
  quantityDelivered: number;
  unitPrice: number | null;
  notes: string | null;
}

export interface Delivery {
  id: number;
  orderId: number;
  deliveryDate: string;
  deliveryNote: string | null;
  notes: string | null;
}

// Sprint F: License Management

export interface LicenseRecord {
  id: number;
  name: string;
  licenseType: string | null;
  validFrom: string | null;
  validUntil: string | null;
  maxUses: number | null;
  currentUses: number;
  commercialAllowed: boolean;
  costPerPiece: number;
  costPerSeries: number;
  costFlat: number;
  source: string | null;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

// Phase 3: Quality & Reporting

export interface QualityInspection {
  id: number;
  projectId: number;
  workflowStepId: number | null;
  inspector: string | null;
  inspectionDate: string;
  result: string;
  notes: string | null;
  createdAt: string;
}

export interface DefectRecord {
  id: number;
  inspectionId: number;
  description: string;
  severity: string | null;
  status: string | null;
  resolvedAt: string | null;
  notes: string | null;
  createdAt: string;
}

export interface AuditLogEntry {
  id: number;
  entityType: string;
  entityId: number;
  fieldName: string;
  oldValue: string | null;
  newValue: string | null;
  changedBy: string | null;
  changedAt: string;
}

export interface CostRate {
  id: number;
  rateType: string;
  name: string;
  rateValue: number;
  unit: string | null;
  setupCost: number;
  notes: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CostBreakdown {
  projectId: number;
  projectName: string;
  quantity: number;
  materialCost: number;
  licenseCost: number;
  stitchCost: number;
  laborCost: number;
  machineCost: number;
  procurementCost: number;
  herstellkosten: number;
  overheadPct: number;
  overheadCost: number;
  selbstkosten: number;
  profitMarginPct: number;
  profitAmount: number;
  nettoVerkaufspreis: number;
  selbstkostenPerPiece: number;
  verkaufspreisPerPiece: number;
}

export interface ProjectReport {
  projectId: number;
  projectName: string;
  totalPlannedMinutes: number;
  totalActualMinutes: number;
  materialCost: number;
  laborCost: number;
  totalCost: number;
  inspectionCount: number;
  passCount: number;
  failCount: number;
  openDefects: number;
  workflowTotal: number;
  workflowCompleted: number;
  costBreakdown: CostBreakdown | null;
}

export interface PrinterInfo {
  name: string;
  displayName: string;
  isDefault: boolean;
}

export interface PrintSettings {
  printerName: string | null;
  paperSize: string;
  orientation: string;
  copies: number;
  scale: number;
  fitToPage: boolean;
  pageRanges: string | null;
  tileEnabled: boolean;
  tileOverlapMm: number;
}

export interface TileInfo {
  sourcePage: number;
  cols: number;
  rows: number;
  totalTiles: number;
  tileWidthMm: number;
  tileHeightMm: number;
}

export interface PdfLayer {
  id: string;
  name: string;
  visible: boolean;
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
  expandedFolderIds: number[];
}

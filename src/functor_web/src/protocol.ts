export type DocumentLanguage = string;

export interface LanguageModeOption {
  id: DocumentLanguage;
  label: string;
}

export interface DocumentSnapshot {
  documentId: number;
  viewId: number;
  title: string;
  isUntitled: boolean;
  text: string;
  language: DocumentLanguage;
  revision: number;
  dirty: boolean;
  cursor: MonacoPosition;
  selection: MonacoSelection | null;
}

export interface WorkspaceSearchHit {
  relativePath: string;
  documentId: number | null;
  revision: number | null;
  lineNumber: number;
  column: number;
  matchLength: number;
  rangeOffset: number;
  rangeLength: number;
  matchedText: string;
  sourceFingerprint: string | null;
  workspaceRoot: string | null;
  preview: string;
}

export interface SearchWorkspaceRequest {
  searchId: string;
  query: string;
  caseSensitive: boolean;
  wholeWord: boolean;
}

export interface OpenWorkspaceSearchResultRequest {
  relativePath: string;
  lineNumber: number;
  column: number;
  matchLength: number;
  documentId: number | null;
  expectedRevision: number | null;
  sourceFingerprint: string | null;
  workspaceRoot: string | null;
}

export interface WorkspaceSearchResponse {
  searchId: string;
  workspaceRoot: string | null;
  hits: WorkspaceSearchHit[];
  cancelled: boolean;
  skippedFiles: number;
  warnings: string[];
}

export type ThemePreset = "graphiteDark" | "graphiteLight" | "custom";

export interface SettingsColors {
  background: string;
  foreground: string;
  selection: string;
  cursor: string;
  lineNumber: string;
  gutterBackground: string;
  gutterSeparator: string;
  editorBorder: string;
  diagnosticError: string;
  diagnosticWarning: string;
  diagnosticInfo: string;
  syntaxKeyword: string;
  syntaxString: string;
  syntaxComment: string;
  syntaxNumber: string;
  syntaxType: string;
  syntaxFunction: string;
  resizeHandleColor: string;
  commandPaletteShadowColor: string;
  workspaceSeparatorColor: string;
  measurementColor: string;
}

export interface SettingsTypography {
  editorFontFamily: string;
  editorFallbackFontFamily: string;
  uiFontFamily: string;
  workspaceFontSize: number;
  editorFontSize: number;
  editorLineHeight: number;
  editorTabSize: number;
  commandPaletteFontSize: number;
  welcomeTitleFontSize: number;
}

export interface SettingsGeometry {
  shellPaddingX: number;
  shellPaddingY: number;
  workspaceSidebarWidth: number;
  editorBorderWidth: number;
  documentTabMinHeight: number;
  documentTabPaddingHorizontal: number;
  documentTabPaddingVertical: number;
  commandPaletteWidth: number;
  commandPalettePadding: number;
  commandPaletteMaxHeight: number;
}

export interface AppSettings {
  schemaVersion: number;
  preset: ThemePreset;
  colors: SettingsColors;
  typography: SettingsTypography;
  geometry: SettingsGeometry;
}

export interface UserThemeSummary {
  name: string;
  preset: ThemePreset;
}

export interface SettingsCatalogResponse {
  settings: AppSettings;
  presets: AppSettings[];
  userThemes: UserThemeSummary[];
  warnings: string[];
}

export interface MonacoPosition {
  lineNumber: number;
  column: number;
}

export interface MonacoSelection {
  anchor: MonacoPosition;
  focus: MonacoPosition;
}

export interface WorkspaceSnapshot {
  documents: DocumentSnapshot[];
  activeViewId: number;
  workspace: WorkspaceStateSnapshot | null;
  workspaceError: string | null;
  persistenceWarning: string | null;
  canReopenClosedDocument: boolean;
}

export interface RestoreWorkspaceResponse {
  snapshot: WorkspaceSnapshot;
  warnings: string[];
}

export interface SaveRecoverySnapshotResponse {
  savedRevision: number | null;
  stale: boolean;
}

export interface RecoveryCandidate {
  recoveryId: string;
  title: string;
  path: string | null;
}

export interface RecoveryCandidatesResponse {
  candidates: RecoveryCandidate[];
  warnings: string[];
}

export interface RecoveryRestoreResponse {
  snapshot: WorkspaceSnapshot;
  warnings: string[];
}

export interface WorkspaceStateSnapshot {
  root: string;
  name: string;
  entries: WorkspaceEntrySnapshot[];
}

export interface WorkspaceEntrySnapshot {
  name: string;
  relativePath: string;
  kind: "file" | "directory";
  children: WorkspaceEntrySnapshot[] | null;
}

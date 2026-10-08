import type {
  AppSettings,
  DocumentLanguage,
  LanguageModeOption,
  SearchWorkspaceRequest,
  SettingsCatalogResponse,
  WorkspaceSearchHit,
  WorkspaceSearchResponse,
  WorkspaceSnapshot,
  UserThemeSummary,
} from "./protocol.js";

export type EditorAction =
  | "new"
  | "open"
  | "openWorkspace"
  | "closeWorkspace"
  | "reopenClosed"
  | "clearRecentDocuments"
  | "save"
  | "saveAs"
  | "exit";

export interface EditorBridgeCallbacks {
  onSnapshot: (snapshot: WorkspaceSnapshot) => void;
  onLanguageOptions: (options: LanguageModeOption[]) => void;
  onStatus: (status: string) => void;
  onDocumentStatus: (status: string) => void;
  onOperationInProgress: (inProgress: boolean) => void;
  onWorkspaceActionNeedsSave: (action: "open" | "close" | null) => void;
}

export interface EditorBridge {
  runAction(action: EditorAction): void;
  switchDocument(viewId: number): void;
  setDocumentLanguage(documentId: number, language: DocumentLanguage): Promise<boolean>;
  saveDocument(documentId: number): Promise<boolean>;
  saveAllDocuments(): Promise<boolean>;
  closeDocument(viewId: number, discardChanges: boolean): Promise<boolean>;
  searchWorkspace(request: SearchWorkspaceRequest): Promise<WorkspaceSearchResponse | null>;
  cancelSearch(searchId: string): Promise<boolean>;
  getSettings(): Promise<SettingsCatalogResponse>;
  saveSettings(settings: AppSettings): Promise<AppSettings>;
  loadUserTheme(name: string): Promise<AppSettings>;
  saveUserTheme(name: string, settings: AppSettings): Promise<UserThemeSummary>;
  openWorkspaceSearchResult(hit: WorkspaceSearchHit): Promise<boolean>;
  setSearchHighlights(hits: WorkspaceSearchHit[] | null, activeIndex: number): void;
  replaceSearchResults(
    hits: WorkspaceSearchHit[],
    replacement: string,
  ): Promise<{ replaced: number } | null>;
  loadWorkspaceDirectory(relativePath: string): Promise<boolean>;
  openWorkspaceFile(relativePath: string): Promise<boolean>;
  dispose(): void;
}

export function createEditorBridge(
  editorElement: HTMLDivElement,
  callbacks: EditorBridgeCallbacks,
): EditorBridge;

import { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";

import { WorkspaceShell } from "functor_ui";
import { createEditorBridge } from "./bridge.js";
import type {
  AppSettings,
  LanguageModeOption,
  SettingsCatalogResponse,
  UserThemeSummary,
  WorkspaceSearchHit,
  WorkspaceSnapshot,
} from "./protocol.js";
import "./style.css";

function App() {
  const editorRef = useRef<HTMLDivElement>(null);
  const bridgeRef = useRef<ReturnType<typeof createEditorBridge> | null>(null);
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const [languageOptions, setLanguageOptions] = useState<LanguageModeOption[]>([]);
  const [status, setStatus] = useState("Starting editor session...");
  const [documentStatus, setDocumentStatus] = useState("Connecting to Rust...");
  const [operationInProgress, setOperationInProgress] = useState(false);
  const [workspaceActionNeedsSave, setWorkspaceActionNeedsSave] = useState<
    "open" | "close" | null
  >(null);
  const [welcomeDismissed, setWelcomeDismissed] = useState(false);
  const getSettings = useCallback((): Promise<SettingsCatalogResponse> => {
    const bridge = bridgeRef.current;
    return bridge
      ? bridge.getSettings()
      : Promise.reject(new Error("The settings bridge is not ready."));
  }, []);
  const saveSettings = useCallback((settings: AppSettings): Promise<AppSettings> => {
    const bridge = bridgeRef.current;
    return bridge
      ? bridge.saveSettings(settings)
      : Promise.reject(new Error("The settings bridge is not ready."));
  }, []);
  const loadUserTheme = useCallback((name: string): Promise<AppSettings> => {
    const bridge = bridgeRef.current;
    return bridge
      ? bridge.loadUserTheme(name)
      : Promise.reject(new Error("The settings bridge is not ready."));
  }, []);
  const saveUserTheme = useCallback(
    (name: string, settings: AppSettings): Promise<UserThemeSummary> => {
      const bridge = bridgeRef.current;
      return bridge
        ? bridge.saveUserTheme(name, settings)
        : Promise.reject(new Error("The settings bridge is not ready."));
    },
    [],
  );
  const setSearchHighlights = useCallback((
    hits: WorkspaceSearchHit[] | null,
    activeIndex: number,
  ) => {
    bridgeRef.current?.setSearchHighlights(hits, activeIndex);
  }, []);

  useEffect(() => {
    if (!editorRef.current) {
      throw new Error("The Monaco editor container was not mounted.");
    }
    const bridge = createEditorBridge(editorRef.current, {
      onSnapshot: setSnapshot,
      onLanguageOptions: setLanguageOptions,
      onStatus: setStatus,
      onDocumentStatus: setDocumentStatus,
      onOperationInProgress: setOperationInProgress,
      onWorkspaceActionNeedsSave: setWorkspaceActionNeedsSave,
    });
    bridgeRef.current = bridge;
    return () => {
      bridge.dispose();
      bridgeRef.current = null;
    };
  }, []);

  const activeDocument = snapshot?.documents.find(
    (document) => document.viewId === snapshot.activeViewId,
  );
  const showWelcome = Boolean(
    !welcomeDismissed
    && snapshot?.documents.length === 1
    && activeDocument?.isUntitled
    && activeDocument.text === ""
    && !activeDocument.dirty,
  );

  return (
    <WorkspaceShell
      documents={snapshot?.documents ?? []}
      activeViewId={snapshot?.activeViewId ?? null}
      activeTitle={activeDocument?.title ?? "Loading..."}
      activeDocument={activeDocument ?? null}
      languageOptions={languageOptions}
      canReopenClosedDocument={snapshot?.canReopenClosedDocument ?? false}
      showWelcome={showWelcome}
      workspace={snapshot?.workspace ?? null}
      workspaceError={snapshot?.workspaceError ?? null}
      status={status}
      documentStatus={documentStatus}
      operationInProgress={operationInProgress}
      workspaceActionNeedsSave={workspaceActionNeedsSave}
      editorRef={editorRef}
      onAction={(action) => {
        if (action === "new") setWelcomeDismissed(true);
        if (action === "openWorkspace" || action === "closeWorkspace") {
          setWelcomeDismissed(false);
        }
        bridgeRef.current?.runAction(action);
      }}
      onSwitchDocument={(viewId) => bridgeRef.current?.switchDocument(viewId)}
      onChangeLanguage={(documentId, language) =>
        bridgeRef.current?.setDocumentLanguage(documentId, language) ?? Promise.resolve(false)}
      onSaveDocument={(documentId) =>
        bridgeRef.current?.saveDocument(documentId) ?? Promise.resolve(false)}
      onSaveAllDocuments={() =>
        bridgeRef.current?.saveAllDocuments() ?? Promise.resolve(false)}
      onDismissWorkspaceActionPrompt={() => setWorkspaceActionNeedsSave(null)}
      onCloseDocument={async (viewId, discardChanges) => {
        const closed = await (
          bridgeRef.current?.closeDocument(viewId, discardChanges) ?? Promise.resolve(false)
        );
        if (closed) setWelcomeDismissed(false);
        return closed;
      }}
      onSearchWorkspace={(request) =>
        bridgeRef.current?.searchWorkspace(request) ?? Promise.resolve(null)}
      onCancelSearch={(searchId) =>
        bridgeRef.current?.cancelSearch(searchId) ?? Promise.resolve(false)}
      onOpenWorkspaceSearchResult={(hit) =>
        bridgeRef.current?.openWorkspaceSearchResult(hit) ?? Promise.resolve(false)}
      onSetSearchHighlights={setSearchHighlights}
      onReplaceSearchResults={(hits, replacement) =>
        bridgeRef.current?.replaceSearchResults(hits, replacement) ?? Promise.resolve(null)}
      onLoadWorkspaceDirectory={(relativePath) =>
        bridgeRef.current?.loadWorkspaceDirectory(relativePath) ?? Promise.resolve(false)}
      onOpenWorkspaceFile={(relativePath) =>
        bridgeRef.current?.openWorkspaceFile(relativePath) ?? Promise.resolve(false)}
      onLoadSettings={getSettings}
      onApplySettings={saveSettings}
      onLoadUserTheme={loadUserTheme}
      onSaveUserTheme={saveUserTheme}
    />
  );
}

const root = document.getElementById("root");
if (!root) {
  throw new Error("The application root element is missing.");
}
createRoot(root).render(<App />);

import { useEffect, useMemo, useRef, useState } from "react";
import {
  settingsColorFromPicker,
  settingsColorPickerValue,
  settingsFontFamilies,
  settingsWithPreset,
} from "./settings_draft.js";
import { validateSettings } from "./settings_validation.js";

interface WorkspaceEntrySnapshot {
  name: string;
  relativePath: string;
  kind: "file" | "directory";
  children: WorkspaceEntrySnapshot[] | null;
}

type ContextMenuState =
  | {
      kind: "workspace-entry";
      entry: WorkspaceEntrySnapshot;
      left: number;
      top: number;
    }
  | { kind: "editor"; left: number; top: number };

interface WorkspaceStateSnapshot {
  root: string;
  name: string;
  entries: WorkspaceEntrySnapshot[];
}

export interface WorkspaceTab {
  documentId: number;
  viewId: number;
  title: string;
  revision: number;
  dirty: boolean;
}

interface WorkspaceStatus {
  documentId: number;
  language: string;
  cursor: { lineNumber: number; column: number };
  dirty: boolean;
}

export interface LanguageModeOption {
  id: string;
  label: string;
}

export function languageLabel(language: string | undefined) {
  switch (language) {
    case "csharp":
      return "C#";
    case "fsharp":
      return "F#";
    case "json":
      return "JSON";
    case "markdown":
      return "Markdown";
    case "plainText":
      return "Plain Text";
    case "rust":
      return "Rust";
    default:
      return language
        ? language[0].toLocaleUpperCase() + language.slice(1)
        : "No language";
  }
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

export interface WorkspaceSearchRequest {
  searchId: string;
  query: string;
  caseSensitive: boolean;
  wholeWord: boolean;
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

export interface ShellSettingsColors {
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

export interface ShellAppSettings {
  schemaVersion: number;
  preset: ThemePreset;
  colors: ShellSettingsColors;
  typography: {
    editorFontFamily: string;
    editorFallbackFontFamily: string;
    uiFontFamily: string;
    workspaceFontSize: number;
    editorFontSize: number;
    editorLineHeight: number;
    editorTabSize: number;
    commandPaletteFontSize: number;
    welcomeTitleFontSize: number;
  };
  geometry: {
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
  };
}

export interface ShellSettingsCatalog {
  settings: ShellAppSettings;
  presets: ShellAppSettings[];
  userThemes: Array<{ name: string; preset: ThemePreset }>;
  warnings: string[];
}

export interface WorkspaceShellProps {
  documents: WorkspaceTab[];
  activeViewId: number | null;
  activeTitle: string;
  activeDocument?: WorkspaceStatus | null;
  languageOptions: LanguageModeOption[];
  canReopenClosedDocument?: boolean;
  showWelcome?: boolean;
  workspace: WorkspaceStateSnapshot | null;
  workspaceError: string | null;
  status: string;
  documentStatus: string;
  operationInProgress: boolean;
  workspaceActionNeedsSave: "open" | "close" | null;
  editorRef: { current: HTMLDivElement | null };
  onAction: (
    action:
      | "new"
      | "open"
      | "openWorkspace"
      | "closeWorkspace"
      | "reopenClosed"
      | "clearRecentDocuments"
      | "save"
      | "saveAs"
      | "exit",
  ) => void;
  onSwitchDocument: (viewId: number) => void;
  onChangeLanguage: (documentId: number, language: string) => Promise<boolean>;
  onSaveDocument: (documentId: number) => Promise<boolean>;
  onSaveAllDocuments: () => Promise<boolean>;
  onDismissWorkspaceActionPrompt: () => void;
  onCloseDocument: (viewId: number, discardChanges: boolean) => Promise<boolean>;
  onSearchWorkspace: (
    request: WorkspaceSearchRequest,
  ) => Promise<WorkspaceSearchResponse | null>;
  onCancelSearch: (searchId: string) => Promise<boolean>;
  onOpenWorkspaceSearchResult: (hit: WorkspaceSearchHit) => Promise<boolean>;
  onSetSearchHighlights: (hits: WorkspaceSearchHit[] | null, activeIndex: number) => void;
  onReplaceSearchResults: (
    hits: WorkspaceSearchHit[],
    replacement: string,
  ) => Promise<{ replaced: number } | null>;
  onLoadWorkspaceDirectory: (relativePath: string) => Promise<boolean>;
  onOpenWorkspaceFile: (relativePath: string) => Promise<boolean>;
  onLoadSettings: () => Promise<ShellSettingsCatalog>;
  onApplySettings: (settings: ShellAppSettings) => Promise<ShellAppSettings>;
  onLoadUserTheme: (name: string) => Promise<ShellAppSettings>;
  onSaveUserTheme: (
    name: string,
    settings: ShellAppSettings,
  ) => Promise<{ name: string; preset: ThemePreset }>;
}

interface ShellCommand {
  id: string;
  title: string;
  description: string;
  category: string;
  shortcut?: string;
  enabled: boolean;
  run: () => void;
}

function commandScore(query: string, value: string) {
  const normalizedQuery = query.trim().toLowerCase();
  const normalizedValue = value.toLowerCase();
  if (!normalizedQuery) return 0;
  if (normalizedValue === normalizedQuery) return 0;
  if (normalizedValue.startsWith(normalizedQuery)) return 1;
  if (normalizedValue.includes(normalizedQuery)) return 2;

  let queryIndex = 0;
  let score = 0;
  for (let valueIndex = 0; valueIndex < normalizedValue.length; valueIndex += 1) {
    if (normalizedQuery[queryIndex] === normalizedValue[valueIndex]) {
      score += valueIndex;
      queryIndex += 1;
      if (queryIndex === normalizedQuery.length) return score + 10;
    }
  }
  return null;
}

function CommandBar({
  commands,
  isOpen,
  onOpen,
  onClose,
  purpose = "commands",
}: {
  commands: ShellCommand[];
  isOpen: boolean;
  onOpen: () => void;
  onClose: () => void;
  purpose?: "commands" | "languages";
}) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const commandBarRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const filteredCommands = useMemo(
    () =>
      commands
        .filter((command) => command.enabled)
        .map((command) => ({
          command,
          score: [
            command.title,
            command.category,
            command.description,
            command.shortcut ?? "",
          ]
            .map((value) => commandScore(query, value))
            .filter((score): score is number => score !== null)
            .sort((left, right) => left - right)[0],
        }))
        .filter((item) => item.score !== undefined)
        .sort(
          (left, right) =>
            (left.score ?? Number.MAX_SAFE_INTEGER)
            - (right.score ?? Number.MAX_SAFE_INTEGER),
        )
        .map((item) => item.command),
    [commands, query],
  );

  useEffect(() => setSelectedIndex(0), [query]);

  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setSelectedIndex(0);
      inputRef.current?.focus();
    }
  }, [isOpen, purpose]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    function closeIfOutside(event: Event) {
      if (event.target instanceof Node && !commandBarRef.current?.contains(event.target)) {
        setQuery("");
        onClose();
      }
    }

    document.addEventListener("pointerdown", closeIfOutside);
    document.addEventListener("focusin", closeIfOutside);
    return () => {
      document.removeEventListener("pointerdown", closeIfOutside);
      document.removeEventListener("focusin", closeIfOutside);
    };
  }, [isOpen, onClose]);

  function dismiss() {
    setQuery("");
    onClose();
  }

  function runSelected() {
    const command = filteredCommands[selectedIndex];
    if (command?.enabled) {
      dismiss();
      command.run();
    }
  }

  return (
    <div className="command-bar" ref={commandBarRef}>
      <label className="visually-hidden" htmlFor="command-bar-query">
        {purpose === "languages" ? "Select language mode" : "Search or run a command"}
      </label>
      <input
        aria-activedescendant={
          isOpen && filteredCommands[selectedIndex]
            ? `command-option-${filteredCommands[selectedIndex].id}`
            : undefined
        }
        aria-autocomplete="list"
        aria-controls="command-bar-options"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-label={purpose === "languages" ? "Select language mode" : "Search or run a command"}
        autoComplete="off"
        className="command-bar-input"
        id="command-bar-query"
        onChange={(event) => setQuery(event.currentTarget.value)}
        onFocus={() => {
          if (!isOpen) onOpen();
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            dismiss();
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            setSelectedIndex((index) =>
              filteredCommands.length === 0 ? 0 : (index + 1) % filteredCommands.length);
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setSelectedIndex((index) =>
              filteredCommands.length === 0
                ? 0
                : (index - 1 + filteredCommands.length) % filteredCommands.length);
          } else if (event.key === "Enter") {
            event.preventDefault();
            runSelected();
          }
        }}
        placeholder={purpose === "languages"
          ? "Search language modes..."
          : "Search or run a command..."}
        ref={inputRef}
        role="combobox"
        type="search"
        value={query}
      />
      {isOpen && (
        <ul
          id="command-bar-options"
          aria-label={purpose === "languages" ? "Available language modes" : "Available commands"}
          className="command-list command-bar-list"
          role="listbox"
        >
          {filteredCommands.map((command, index) => (
            <li key={command.id} role="presentation">
              <button
                aria-disabled={!command.enabled}
                aria-selected={index === selectedIndex}
                className="command-option"
                id={`command-option-${command.id}`}
                onClick={() => {
                  if (command.enabled) {
                    dismiss();
                    command.run();
                  }
                }}
                onMouseEnter={() => setSelectedIndex(index)}
                role="option"
                type="button"
              >
                <span className="command-option-main">
                  <span>{command.title}</span>
                  <span className="command-category">{command.category}</span>
                </span>
                <span className="command-option-detail">
                  <span>{command.description}</span>
                  {command.shortcut && <kbd>{command.shortcut}</kbd>}
                </span>
              </button>
            </li>
          ))}
          {filteredCommands.length === 0 && (
            <li className="command-empty" role="option" aria-selected="false">
              No matching commands.
            </li>
          )}
        </ul>
      )}
    </div>
  );
}

function DocumentTabs({
  documents,
  activeViewId,
  onSwitchDocument,
  onRequestClose,
  operationInProgress,
  settingsTabOpen,
  settingsTabActive,
  settingsBusy,
  onActivateSettings,
  onCloseSettings,
}: {
  documents: WorkspaceTab[];
  activeViewId: number | null;
  onSwitchDocument: (viewId: number) => void;
  onRequestClose: (document: WorkspaceTab) => void;
  operationInProgress: boolean;
  settingsTabOpen: boolean;
  settingsTabActive: boolean;
  settingsBusy: boolean;
  onActivateSettings: () => void;
  onCloseSettings: () => void;
}) {
  return (
    <nav className="tabs" role="tablist" aria-label="Open tabs">
      {documents.map((document) => (
        <div className="tab-item" key={document.viewId} role="presentation">
          <button
            type="button"
            role="tab"
            aria-selected={document.viewId === activeViewId}
            onClick={() => onSwitchDocument(document.viewId)}
          >
            {document.dirty ? "● " : ""}
            {document.title}
          </button>
          <button
            className="tab-close"
            type="button"
            aria-label={`Close ${document.title}`}
            disabled={operationInProgress}
            onClick={() => onRequestClose(document)}
          >
            ×
          </button>
        </div>
      ))}
      {settingsTabOpen && (
        <div className="tab-item" role="presentation">
          <button
            id="settings-tab"
            type="button"
            role="tab"
            aria-selected={settingsTabActive}
            aria-controls="settings-tabpanel"
            onClick={onActivateSettings}
          >
            Settings
          </button>
          <button
            className="tab-close"
            type="button"
            aria-label="Close Settings"
            disabled={operationInProgress || settingsBusy}
            onClick={onCloseSettings}
          >
            ×
          </button>
        </div>
      )}
    </nav>
  );
}

function WorkspaceExplorer({
  workspace,
  workspaceError,
  operationInProgress,
  onLoadDirectory,
  onOpenFile,
  contextMenu,
  onContextMenuEntry,
  onDismissContextMenu,
}: {
  workspace: WorkspaceStateSnapshot | null;
  workspaceError: string | null;
  operationInProgress: boolean;
  onLoadDirectory: (relativePath: string) => Promise<boolean>;
  onOpenFile: (relativePath: string) => Promise<boolean>;
  contextMenu: ContextMenuState | null;
  onContextMenuEntry: (
    event: React.MouseEvent<HTMLButtonElement>,
    entry: WorkspaceEntrySnapshot,
  ) => void;
  onDismissContextMenu: () => void;
}) {
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [loadingPaths, setLoadingPaths] = useState<Set<string>>(new Set());
  const contextMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setExpandedPaths(new Set());
    setLoadingPaths(new Set());
  }, [workspace?.root]);

  useEffect(() => {
    if (contextMenu?.kind === "workspace-entry") {
      contextMenuRef.current?.focus();
    }
  }, [contextMenu]);

  async function expandDirectory(entry: WorkspaceEntrySnapshot) {
    if (expandedPaths.has(entry.relativePath) || loadingPaths.has(entry.relativePath)) {
      return;
    }

    if (entry.children === null) {
      setLoadingPaths((current) => new Set(current).add(entry.relativePath));
      let loaded: boolean;
      try {
        loaded = await onLoadDirectory(entry.relativePath);
      } finally {
        setLoadingPaths((current) => {
          const next = new Set(current);
          next.delete(entry.relativePath);
          return next;
        });
      }
      if (!loaded) {
        return;
      }
    }
    setExpandedPaths((current) => new Set(current).add(entry.relativePath));
  }

  async function toggleDirectory(entry: WorkspaceEntrySnapshot) {
    if (expandedPaths.has(entry.relativePath)) {
      setExpandedPaths((current) => {
        const next = new Set(current);
        next.delete(entry.relativePath);
        return next;
      });
      return;
    }
    await expandDirectory(entry);
  }

  function renderEntry(entry: WorkspaceEntrySnapshot) {
    const isDirectory = entry.kind === "directory";
    const isExpanded = expandedPaths.has(entry.relativePath);
    const isLoading = loadingPaths.has(entry.relativePath);
    return (
      <li key={entry.relativePath}>
        <button
          className="tree-entry"
          disabled={operationInProgress || isLoading}
          onClick={() => {
            if (isDirectory) {
              void toggleDirectory(entry);
            } else {
              void onOpenFile(entry.relativePath);
            }
          }}
          onContextMenu={(event) => {
            onContextMenuEntry(event, entry);
          }}
          type="button"
          aria-expanded={isDirectory ? isExpanded : undefined}
          aria-label={isDirectory ? `${isExpanded ? "Collapse" : "Expand"} ${entry.name}` : `Open ${entry.name}`}
          data-context-menu-surface="workspace-entry"
          title={entry.relativePath}
        >
          <span className={`tree-entry-icon ${entry.kind}`} aria-hidden="true" />
          <span className="tree-entry-name">{entry.name}</span>
          {isLoading && <span className="tree-entry-loading">Loading</span>}
        </button>
        {isExpanded && isDirectory && entry.children && (
          <ul>
            {entry.children.length > 0
              ? entry.children.map(renderEntry)
              : <li className="tree-empty">Empty folder</li>}
          </ul>
        )}
      </li>
    );
  }

  const contextDirectoryExpanded = contextMenu?.kind === "workspace-entry"
    && contextMenu.entry.kind === "directory"
    && expandedPaths.has(contextMenu.entry.relativePath);

  return (
    <aside className="workspace-explorer" aria-label="Project files">
      <h2>Project</h2>
      {workspaceError && <p className="workspace-error" role="alert">{workspaceError}</p>}
      {workspace ? (
        <>
          <p className="workspace-root" title={workspace.root}>{workspace.name}</p>
          {workspace.entries.length > 0 ? (
            <ul className="workspace-tree" aria-label={`${workspace.name} files`}>
              {workspace.entries.map(renderEntry)}
            </ul>
          ) : (
            <p className="tree-empty">This folder is empty.</p>
          )}
        </>
      ) : (
        <p className="workspace-empty">
          Open a folder to browse and organize project files.
        </p>
      )}
      {contextMenu?.kind === "workspace-entry" && (
        <div
          aria-label={`${contextMenu.entry.kind === "directory" ? "Folder" : "File"} actions for ${contextMenu.entry.name}`}
          className="context-menu"
          data-context-menu-surface="context-menu"
          ref={contextMenuRef}
          role="menu"
          style={{ left: contextMenu.left, top: contextMenu.top }}
          tabIndex={-1}
        >
          {contextMenu.entry.kind === "directory" ? (
            <button
              disabled={operationInProgress || loadingPaths.has(contextMenu.entry.relativePath)}
              onClick={() => {
                onDismissContextMenu();
                void toggleDirectory(contextMenu.entry);
              }}
              role="menuitem"
              type="button"
            >
              {contextDirectoryExpanded ? "Collapse folder" : "Expand folder"}
            </button>
          ) : (
            <button
              disabled={operationInProgress}
              onClick={() => {
                onDismissContextMenu();
                void onOpenFile(contextMenu.entry.relativePath);
              }}
              role="menuitem"
              type="button"
            >
              Open file
            </button>
          )}
        </div>
      )}
    </aside>
  );
}

function WorkspaceSearch({
  workspace,
  documents,
  operationInProgress,
  onSearch,
  onCancel,
  onOpenResult,
  onSetHighlights,
  onReplace,
}: {
  workspace: WorkspaceStateSnapshot | null;
  documents: WorkspaceTab[];
  operationInProgress: boolean;
  onSearch: (request: WorkspaceSearchRequest) => Promise<WorkspaceSearchResponse | null>;
  onCancel: (searchId: string) => Promise<boolean>;
  onOpenResult: (hit: WorkspaceSearchHit) => Promise<boolean>;
  onSetHighlights: (hits: WorkspaceSearchHit[] | null, activeIndex: number) => void;
  onReplace: (
    hits: WorkspaceSearchHit[],
    replacement: string,
  ) => Promise<{ replaced: number } | null>;
}) {
  const [query, setQuery] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [searching, setSearching] = useState(false);
  const [activeSearchId, setActiveSearchId] = useState<string | null>(null);
  const [response, setResponse] = useState<WorkspaceSearchResponse | null>(null);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [replacement, setReplacement] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");

  function invalidateResults() {
    setResponse(null);
    setActiveIndex(-1);
    setNotice("");
    setError("");
    onSetHighlights(null, -1);
  }

  useEffect(() => {
    setResponse(null);
    setActiveIndex(-1);
    onSetHighlights(null, -1);
  }, [workspace?.root]);

  const stale = Boolean(response?.hits.some((hit) => {
    if (hit.documentId === null) return false;
    return documents.find((document) => document.documentId === hit.documentId)?.revision
      !== hit.revision;
  }));

  useEffect(() => {
    if (stale) {
      onSetHighlights(null, -1);
    } else {
      onSetHighlights(response?.hits ?? null, activeIndex);
    }
  }, [response, activeIndex, stale, onSetHighlights]);

  async function submitSearch(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const term = query.trim();
    if (
      !term
      || searching
      || operationInProgress
    ) {
      return;
    }
    const request: WorkspaceSearchRequest = {
      searchId: globalThis.crypto.randomUUID(),
      query: term,
      caseSensitive,
      wholeWord,
    };
    setSearching(true);
    setActiveSearchId(request.searchId);
    setResponse(null);
    setActiveIndex(-1);
    setError("");
    setNotice("");
    onSetHighlights(null, -1);
    try {
      const result = await onSearch(request);
      if (!result) {
        setError("Search did not complete.");
        return;
      }
      setResponse(result);
      setActiveIndex(result.hits.length > 0 ? 0 : -1);
      if (result.cancelled) {
        setNotice("Search cancelled.");
      }
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setSearching(false);
      setActiveSearchId(null);
    }
  }

  async function cancelSearch() {
    if (!activeSearchId) return;
    try {
      const cancelled = await onCancel(activeSearchId);
      setNotice(cancelled ? "Cancelling search..." : "Search already completed.");
    } catch (caught) {
      setError(`Search could not be cancelled: ${errorMessage(caught)}`);
    }
  }

  async function navigateTo(index: number) {
    if (!response || stale || response.hits.length === 0) return;
    const nextIndex = (index + response.hits.length) % response.hits.length;
    setActiveIndex(nextIndex);
    const opened = await onOpenResult(response.hits[nextIndex]);
    if (!opened) {
      setError("The selected result is stale or could not be opened. Search again.");
    }
  }

  async function replaceMatches(matches: WorkspaceSearchHit[]) {
    setError("");
    setNotice("");
    try {
      const result = await onReplace(matches, replacement);
      if (!result) return;
      setNotice(`Replaced ${result.replaced} match${result.replaced === 1 ? "" : "es"}.`);
      setResponse(null);
      setActiveIndex(-1);
      onSetHighlights(null, -1);
    } catch (caught) {
      setError(`Replacement failed: ${errorMessage(caught)}`);
    }
  }

  const canSearch = query.trim().length > 0
    && !operationInProgress
    && !searching;
  const canReplaceAll = Boolean(
    response
    && !response.cancelled
    && response.hits.length > 0
    && !stale
    && !operationInProgress
    && !searching,
  );

  return (
    <aside className="workspace-search" aria-label="Search documents and project files">
      <h2>Search</h2>
      <form className="workspace-search-form" onSubmit={(event) => void submitSearch(event)}>
        <label htmlFor="workspace-search-query">Search text</label>
        <input
          autoComplete="off"
          id="workspace-search-query"
          autoFocus
          disabled={searching || operationInProgress}
          maxLength={256}
          onChange={(event) => {
            setQuery(event.currentTarget.value);
            invalidateResults();
          }}
          type="search"
          value={query}
        />
        <label htmlFor="workspace-search-replacement">Replacement text</label>
        <input
          autoComplete="off"
          id="workspace-search-replacement"
          disabled={searching || operationInProgress}
          maxLength={100_000}
          onChange={(event) => setReplacement(event.currentTarget.value)}
          value={replacement}
        />
        <label className="case-sensitive-option">
          <input
            checked={caseSensitive}
            disabled={searching || operationInProgress}
            onChange={(event) => {
              setCaseSensitive(event.currentTarget.checked);
              invalidateResults();
            }}
            type="checkbox"
          />
          Match case
        </label>
        <label className="case-sensitive-option">
          <input
            checked={wholeWord}
            disabled={searching || operationInProgress}
            onChange={(event) => {
              setWholeWord(event.currentTarget.checked);
              invalidateResults();
            }}
            type="checkbox"
          />
          Whole word
        </label>
        <div className="search-actions">
          <button disabled={!canSearch} type="submit">
            {searching ? "Searching..." : "Search"}
          </button>
          <button
            disabled={!canReplaceAll}
            onClick={() => response && void replaceMatches(response.hits)}
            type="button"
          >
            Replace All
          </button>
          {searching && (
            <button onClick={() => void cancelSearch()} type="button">
              Cancel
            </button>
          )}
        </div>
      </form>
      {error && <p className="workspace-error" role="alert">{error}</p>}
      {notice && <p className="search-summary" role="status">{notice}</p>}
      {response && (
        <>
          <p className="search-summary" role="status">
            {response.cancelled
              ? "Search cancelled."
              : `${response.hits.length} match${response.hits.length === 1 ? "" : "es"}`
              + (response.skippedFiles > 0
                ? ` · ${response.skippedFiles} non-text or unreadable files skipped`
                : "")}
          </p>
          {stale && (
            <p className="workspace-error" role="alert">
              Search results are stale because a document changed or closed. Search again.
            </p>
          )}
          {response.warnings.length > 0 && (
            <ul className="search-warnings" aria-label="Search warnings">
              {response.warnings.map((warning, index) => (
                <li key={`${index}-${warning}`}>{warning}</li>
              ))}
            </ul>
          )}
          {response.hits.length > 0 && (
            <>
              <div className="search-navigation" aria-label="Match navigation">
                <button
                  aria-label="Previous match"
                  disabled={stale || operationInProgress}
                  onClick={() => void navigateTo(activeIndex - 1)}
                  type="button"
                >
                  Previous
                </button>
                <span>{activeIndex + 1} of {response.hits.length}</span>
                <button
                  aria-label="Next match"
                  disabled={stale || operationInProgress}
                  onClick={() => void navigateTo(activeIndex + 1)}
                  type="button"
                >
                  Next
                </button>
              </div>
              <ol className="search-results" aria-label="Search results">
                {response.hits.map((hit, index) => (
                  <li key={`${hit.documentId ?? hit.relativePath}:${hit.lineNumber}:${hit.column}:${index}`}>
                    <button
                      aria-current={index === activeIndex ? "true" : undefined}
                      className={`search-result${index === activeIndex ? " search-result-active" : ""}`}
                      disabled={stale || operationInProgress}
                      onClick={() => void navigateTo(index)}
                      type="button"
                    >
                      <span className="search-result-location">
                        {hit.relativePath}:{hit.lineNumber}:{hit.column}
                      </span>
                      <span className="search-result-preview">{hit.preview}</span>
                    </button>
                  </li>
                ))}
              </ol>
            </>
          )}
          {!response.cancelled && response.hits.length === 0 && (
            <p className="tree-empty">No matches found.</p>
          )}
        </>
      )}
    </aside>
  );
}

const settingsColorFields = [
  ["background", "Editor background"],
  ["foreground", "Editor foreground"],
  ["selection", "Selection"],
  ["cursor", "Cursor and accent"],
  ["lineNumber", "Line numbers and muted text"],
  ["gutterBackground", "Gutter and panel background"],
  ["gutterSeparator", "Gutter separator"],
  ["editorBorder", "Editor border"],
  ["diagnosticError", "Diagnostic error"],
  ["diagnosticWarning", "Diagnostic warning"],
  ["diagnosticInfo", "Diagnostic information"],
  ["syntaxKeyword", "Syntax keyword"],
  ["syntaxString", "Syntax string"],
  ["syntaxComment", "Syntax comment"],
  ["syntaxNumber", "Syntax number"],
  ["syntaxType", "Syntax type"],
  ["syntaxFunction", "Syntax function"],
  ["resizeHandleColor", "Resize handle"],
  ["commandPaletteShadowColor", "Command palette shadow"],
  ["workspaceSeparatorColor", "Workspace separator"],
  ["measurementColor", "Measurement"],
] as const satisfies ReadonlyArray<[keyof ShellSettingsColors, string]>;

const typographyNumberFields = [
  ["workspaceFontSize", "Workspace font size", "1"],
  ["editorFontSize", "Editor font size", "0.5"],
  ["editorLineHeight", "Editor line height", "1"],
  ["editorTabSize", "Editor tab size", "1"],
  ["commandPaletteFontSize", "Command palette font size", "1"],
  ["welcomeTitleFontSize", "Welcome title font size", "1"],
] as const;

const geometryNumberFields = [
  ["shellPaddingX", "Horizontal shell padding", "1"],
  ["shellPaddingY", "Vertical shell padding", "1"],
  ["workspaceSidebarWidth", "Workspace sidebar width", "1"],
  ["editorBorderWidth", "Editor border width", "0.5"],
  ["documentTabMinHeight", "Document tab height", "1"],
  ["documentTabPaddingHorizontal", "Document tab horizontal padding", "1"],
  ["documentTabPaddingVertical", "Document tab vertical padding", "1"],
  ["commandPaletteWidth", "Command palette width", "1"],
  ["commandPalettePadding", "Command palette padding", "1"],
  ["commandPaletteMaxHeight", "Command palette maximum height", "1"],
] as const;

function isThemePreset(value: string): value is ThemePreset {
  return value === "graphiteDark" || value === "graphiteLight" || value === "custom";
}

function SettingsDialog({
  onBusyChange,
  onLoadSettings,
  onApplySettings,
  onLoadUserTheme,
  onSaveUserTheme,
}: {
  onBusyChange: (busy: boolean) => void;
  onLoadSettings: () => Promise<ShellSettingsCatalog>;
  onApplySettings: (settings: ShellAppSettings) => Promise<ShellAppSettings>;
  onLoadUserTheme: (name: string) => Promise<ShellAppSettings>;
  onSaveUserTheme: (
    name: string,
    settings: ShellAppSettings,
  ) => Promise<{ name: string; preset: ThemePreset }>;
}) {
  const [catalog, setCatalog] = useState<ShellSettingsCatalog | null>(null);
  const [draft, setDraft] = useState<ShellAppSettings | null>(null);
  const [selectedTheme, setSelectedTheme] = useState("");
  const [newThemeName, setNewThemeName] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => onBusyChange(busy), [busy, onBusyChange]);

  useEffect(() => {
    let mounted = true;
    onLoadSettings()
      .then((loaded) => {
        if (!mounted) return;
        setCatalog(loaded);
        setDraft(loaded.settings);
      })
      .catch((error: unknown) => {
        if (mounted) setErrors([`Settings could not be loaded: ${errorMessage(error)}`]);
      });
    return () => {
      mounted = false;
    };
  }, [onLoadSettings]);

  function updateTypographyFamily(
    field: "editorFontFamily" | "editorFallbackFontFamily" | "uiFontFamily",
    value: string,
  ) {
    setDraft((current) =>
      current
        ? { ...current, typography: { ...current.typography, [field]: value } }
        : current);
  }

  function updateTypographyNumber(
    field: "workspaceFontSize" | "editorFontSize" | "editorLineHeight"
      | "editorTabSize" | "commandPaletteFontSize" | "welcomeTitleFontSize",
    value: number,
  ) {
    setDraft((current) =>
      current
        ? { ...current, typography: { ...current.typography, [field]: value } }
        : current);
  }

  function updateGeometry(
    field: keyof ShellAppSettings["geometry"],
    value: number,
  ) {
    setDraft((current) =>
      current
        ? { ...current, geometry: { ...current.geometry, [field]: value } }
        : current);
  }

  function updateColor(field: keyof ShellSettingsColors, value: string) {
    setDraft((current) =>
      current
        ? { ...current, colors: { ...current.colors, [field]: value } }
        : current);
  }

  function updateColorFromPicker(field: keyof ShellSettingsColors, value: string) {
    setDraft((current) => {
      if (!current) return current;
      const color = settingsColorFromPicker(current.colors[field], value);
      return color
        ? { ...current, colors: { ...current.colors, [field]: color } }
        : current;
    });
  }

  function selectPreset(value: string) {
    if (!isThemePreset(value) || !draft) {
      return;
    }
    const settings = settingsWithPreset(draft, catalog?.presets ?? [], value);
    if (settings) {
      setDraft(settings);
      setErrors([]);
      setMessage(
        value === "custom"
          ? "Custom settings are ready to edit."
          : `${value === "graphiteLight" ? "Graphite Light" : "Graphite Dark"} loaded as a draft.`,
      );
    }
  }

  async function applySettings(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!draft) return;
    const validationErrors = validateSettings(draft);
    if (validationErrors.length > 0) {
      setErrors(validationErrors);
      setMessage("");
      return;
    }
    setBusy(true);
    setErrors([]);
    setMessage("");
    try {
      const saved = await onApplySettings(draft);
      setDraft(saved);
      setMessage("Settings applied and saved.");
    } catch (error) {
      setErrors([`Settings could not be applied: ${errorMessage(error)}`]);
    } finally {
      setBusy(false);
    }
  }

  async function loadSelectedTheme() {
    if (!selectedTheme) return;
    setBusy(true);
    setErrors([]);
    setMessage("");
    try {
      setDraft(await onLoadUserTheme(selectedTheme));
      setMessage(`Theme "${selectedTheme}" loaded as a draft. Apply settings to use it.`);
    } catch (error) {
      setErrors([`Theme could not be loaded: ${errorMessage(error)}`]);
    } finally {
      setBusy(false);
    }
  }

  async function saveTheme() {
    if (!draft || !catalog) return;
    const validationErrors = validateSettings(draft);
    if (validationErrors.length > 0) {
      setErrors(validationErrors);
      setMessage("");
      return;
    }
    setBusy(true);
    setErrors([]);
    setMessage("");
    try {
      const theme = await onSaveUserTheme(newThemeName, draft);
      setCatalog({
        ...catalog,
        userThemes: [...catalog.userThemes, theme].sort((left, right) =>
          left.name.localeCompare(right.name)),
      });
      setSelectedTheme(theme.name);
      setNewThemeName("");
      setMessage(`Theme "${theme.name}" saved.`);
    } catch (error) {
      setErrors([`Theme could not be saved: ${errorMessage(error)}`]);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section
      aria-labelledby="settings-title"
      className="settings-dialog"
      role="region"
    >
      <header className="settings-header">
        <div>
          <p className="eyebrow">Appearance and editor behavior</p>
          <h2 id="settings-title">Settings</h2>
        </div>
      </header>
      {!draft ? (
        errors.length > 0
          ? <ul className="settings-errors" role="alert">{errors.map((error) => <li key={error}>{error}</li>)}</ul>
          : <p role="status">Loading settings...</p>
      ) : (
        <form className="settings-form" onSubmit={(event) => void applySettings(event)}>
          {errors.length > 0 && (
            <ul className="settings-errors" role="alert">
              {errors.map((error, index) => <li key={`${index}-${error}`}>{error}</li>)}
            </ul>
          )}
          {catalog?.warnings.map((warning, index) => (
            <p className="settings-warning" key={`${index}-${warning}`} role="status">{warning}</p>
          ))}
          {message && <p className="settings-message" role="status">{message}</p>}
          <section className="settings-section" aria-labelledby="settings-theme-heading">
            <h3 id="settings-theme-heading">Theme</h3>
            <label className="setting-field">
              Preset
              <select
                onChange={(event) => selectPreset(event.currentTarget.value)}
                value={draft.preset}
              >
                <option value="graphiteDark">Graphite Dark</option>
                <option value="graphiteLight">Graphite Light</option>
                <option value="custom">Custom</option>
              </select>
            </label>
            <div className="theme-file-controls">
              <label className="setting-field">
                User theme
                <select
                  onChange={(event) => setSelectedTheme(event.currentTarget.value)}
                  value={selectedTheme}
                >
                  <option value="">Select a saved theme</option>
                  {catalog?.userThemes.map((theme) => (
                    <option key={theme.name} value={theme.name}>{theme.name}</option>
                  ))}
                </select>
              </label>
              <button
                disabled={!selectedTheme || busy}
                onClick={() => void loadSelectedTheme()}
                type="button"
              >
                Load Theme File
              </button>
            </div>
          </section>
          <section className="settings-section" aria-labelledby="settings-type-heading">
            <h3 id="settings-type-heading">Typography</h3>
            <div className="settings-grid">
              {([
                ["uiFontFamily", "UI font family"],
                ["editorFontFamily", "Editor font family"],
                ["editorFallbackFontFamily", "Editor fallback font"],
              ] as const).map(([field, label]) => (
                <label className="setting-field" key={field}>
                  {label}
                  <select
                    onChange={(event) => updateTypographyFamily(field, event.currentTarget.value)}
                    value={draft.typography[field]}
                  >
                    {!settingsFontFamilies.includes(draft.typography[field]) && (
                      <option value={draft.typography[field]}>
                        {draft.typography[field]} (currently selected)
                      </option>
                    )}
                    {settingsFontFamilies.map((fontFamily) => (
                      <option key={fontFamily} value={fontFamily}>{fontFamily}</option>
                    ))}
                  </select>
                </label>
              ))}
              {typographyNumberFields.map(([field, label, step]) => (
                <label className="setting-field" key={field}>
                  {label}
                  <input
                    onChange={(event) => updateTypographyNumber(field, Number(event.currentTarget.value))}
                    step={step}
                    type="number"
                    value={draft.typography[field]}
                  />
                </label>
              ))}
            </div>
          </section>
          <section className="settings-section" aria-labelledby="settings-layout-heading">
            <h3 id="settings-layout-heading">Shell and editor geometry (pixels)</h3>
            <div className="settings-grid">
              {geometryNumberFields.map(([field, label, step]) => (
                <label className="setting-field" key={field}>
                  {label}
                  <input
                    onChange={(event) => updateGeometry(field, Number(event.currentTarget.value))}
                    step={step}
                    type="number"
                    value={draft.geometry[field]}
                  />
                </label>
              ))}
            </div>
          </section>
          <section className="settings-section" aria-labelledby="settings-colors-heading">
            <h3 id="settings-colors-heading">Colors (ARGB hex)</h3>
            <div className="settings-grid settings-color-grid">
              {settingsColorFields.map(([field, label]) => (
                <div className="setting-field" key={field}>
                  <label htmlFor={`settings-color-${field}`}>{label}</label>
                  <div className="settings-color-controls">
                    <input
                      autoCapitalize="off"
                      id={`settings-color-${field}`}
                      maxLength={9}
                      onChange={(event) => updateColor(field, event.currentTarget.value)}
                      spellCheck={false}
                      type="text"
                      value={draft.colors[field]}
                    />
                    <input
                      aria-label={`${label} color picker`}
                      disabled={settingsColorPickerValue(draft.colors[field]) === null}
                      onChange={(event) => updateColorFromPicker(field, event.currentTarget.value)}
                      type="color"
                      value={settingsColorPickerValue(draft.colors[field]) ?? "#000000"}
                    />
                  </div>
                </div>
              ))}
            </div>
          </section>
          <div className="settings-footer">
            <label className="setting-field theme-name-field">
              Save current theme as
              <input
                maxLength={64}
                onChange={(event) => setNewThemeName(event.currentTarget.value)}
                placeholder="Theme name"
                value={newThemeName}
              />
            </label>
            <button disabled={!newThemeName.trim() || busy} onClick={() => void saveTheme()} type="button">
              Save Theme File
            </button>
            <button disabled={busy} type="submit">Apply and Save</button>
          </div>
        </form>
      )}
    </section>
  );
}

function errorMessage(error: unknown) {
  if (error && typeof error === "object" && "message" in error) {
    return String(error.message);
  }
  return String(error);
}

function contextMenuPosition(event: React.MouseEvent<HTMLElement>) {
  const bounds = event.currentTarget.getBoundingClientRect();
  const requestedLeft = event.clientX || bounds.left;
  const requestedTop = event.clientY || bounds.bottom;
  return {
    left: Math.max(8, Math.min(requestedLeft, window.innerWidth - 216)),
    top: Math.max(8, Math.min(requestedTop, window.innerHeight - 64)),
  };
}

export function WorkspaceShell({
  documents,
  activeViewId,
  activeTitle,
  activeDocument = null,
  languageOptions = [],
  canReopenClosedDocument = false,
  showWelcome = false,
  workspace,
  workspaceError,
  status,
  documentStatus,
  operationInProgress,
  workspaceActionNeedsSave,
  editorRef,
  onAction,
  onSwitchDocument,
  onChangeLanguage,
  onSaveDocument,
  onSaveAllDocuments,
  onDismissWorkspaceActionPrompt,
  onCloseDocument,
  onSearchWorkspace,
  onOpenWorkspaceSearchResult,
  onCancelSearch,
  onSetSearchHighlights,
  onReplaceSearchResults,
  onLoadWorkspaceDirectory,
  onOpenWorkspaceFile,
  onLoadSettings,
  onApplySettings,
  onLoadUserTheme,
  onSaveUserTheme,
}: WorkspaceShellProps) {
  const [pendingClose, setPendingClose] = useState<WorkspaceTab | null>(null);
  const [workspacePanel, setWorkspacePanel] = useState<"files" | "search" | "notebook">("files");
  const [commandBarOpen, setCommandBarOpen] = useState(false);
  const [commandBarPurpose, setCommandBarPurpose] = useState<"commands" | "languages">(
    "commands",
  );
  const [fileMenuOpen, setFileMenuOpen] = useState(false);
  const [settingsTabOpen, setSettingsTabOpen] = useState(false);
  const [settingsTabActive, setSettingsTabActive] = useState(false);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [agentPanelOpen, setAgentPanelOpen] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const fileMenuRef = useRef<HTMLDivElement>(null);
  const fileMenuTriggerRef = useRef<HTMLButtonElement>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);
  const contextMenuTriggerRef = useRef<HTMLElement | null>(null);
  const activeTab = showWelcome
    ? null
    : documents.find((document) => document.viewId === activeViewId) ?? null;

  useEffect(() => {
    if (contextMenu?.kind === "editor") {
      contextMenuRef.current?.focus();
    }
  }, [contextMenu]);

  function openCommandBar(purpose: "commands" | "languages" = "commands") {
    setCommandBarPurpose(purpose);
    setCommandBarOpen(true);
  }

  function closeCommandBar() {
    setCommandBarOpen(false);
    setCommandBarPurpose("commands");
  }

  function openSettingsTab() {
    setSettingsTabOpen(true);
    setSettingsTabActive(true);
  }

  function closeSettingsTab() {
    setSettingsTabOpen(false);
    setSettingsTabActive(false);
  }

  function showWorkspaceEntryContextMenu(
    event: React.MouseEvent<HTMLButtonElement>,
    entry: WorkspaceEntrySnapshot,
  ) {
    event.preventDefault();
    const position = contextMenuPosition(event);
    setContextMenu({ kind: "workspace-entry", entry, ...position });
  }

  function showEditorContextMenu(event: React.MouseEvent<HTMLDivElement>) {
    event.preventDefault();
    contextMenuTriggerRef.current = event.target instanceof HTMLElement
      ? event.target
      : event.currentTarget;
    const position = contextMenuPosition(event);
    setContextMenu({ kind: "editor", ...position });
  }

  function handleAppContextMenu(event: React.MouseEvent<HTMLElement>) {
    event.preventDefault();
    const target = event.target;
    if (!(target instanceof Element) || !target.closest("[data-context-menu-surface]")) {
      setContextMenu(null);
    }
  }

  function handleAppKeyDown(event: React.KeyboardEvent<HTMLElement>) {
    if (event.key === "Escape" && contextMenu) {
      event.preventDefault();
      setContextMenu(null);
      contextMenuTriggerRef.current?.focus();
    }
  }

  function runFileAction(
    action: "openWorkspace" | "closeWorkspace" | "new" | "open" | "save" | "saveAs" | "exit",
  ) {
    setFileMenuOpen(false);
    onAction(action);
  }

  async function requestClose(document: WorkspaceTab) {
    if (document.dirty) {
      setPendingClose(document);
      return;
    }
    await onCloseDocument(document.viewId, false);
  }

  async function closeAfterSave() {
    if (!pendingClose) {
      return;
    }
    const saved = await onSaveDocument(pendingClose.documentId);
    if (saved) {
      const closed = await onCloseDocument(pendingClose.viewId, false);
      if (closed) {
        setPendingClose(null);
      }
    }
  }

  async function saveAndContinueWorkspaceAction() {
    const action = workspaceActionNeedsSave;
    if (!action) {
      return;
    }
    if (await onSaveAllDocuments()) {
      onDismissWorkspaceActionPrompt();
      onAction(action === "close" ? "closeWorkspace" : "openWorkspace");
    }
  }

  async function discardAndClose() {
    if (pendingClose) {
      const closed = await onCloseDocument(pendingClose.viewId, true);
      if (closed) {
        setPendingClose(null);
      }
    }
  }

  const commands: ShellCommand[] = [
    {
      id: "file.new",
      title: "New File",
      description: "Create a new untitled document",
      category: "File",
      shortcut: "Ctrl+N",
      enabled: !operationInProgress,
      run: () => onAction("new"),
    },
    {
      id: "file.open",
      title: "Open File",
      description: "Open a document from disk",
      category: "File",
      shortcut: "Ctrl+O",
      enabled: !operationInProgress,
      run: () => onAction("open"),
    },
    {
      id: "workspace.openFolder",
      title: "Open Folder",
      description: "Open a folder as the workspace root",
      category: "Workspace",
      enabled: !operationInProgress,
      run: () => onAction("openWorkspace"),
    },
    {
      id: "workspace.closeFolder",
      title: "Close Folder",
      description: "Close the current workspace folder",
      category: "Workspace",
      enabled: workspace !== null && !operationInProgress,
      run: () => onAction("closeWorkspace"),
    },
    {
      id: "file.save",
      title: "Save File",
      description: "Save the active document",
      category: "File",
      shortcut: "Ctrl+S",
      enabled: activeTab !== null && !operationInProgress,
      run: () => onAction("save"),
    },
    {
      id: "file.saveAs",
      title: "Save File As",
      description: "Save the active document to a new path",
      category: "File",
      shortcut: "Ctrl+Shift+S",
      enabled: !showWelcome && !operationInProgress,
      run: () => onAction("saveAs"),
    },
    {
      id: "file.close",
      title: "Close File",
      description: "Close the active document",
      category: "File",
      enabled: activeTab !== null && !operationInProgress,
      run: () => {
        if (activeTab) void requestClose(activeTab);
      },
    },
    {
      id: "file.reopenClosedTab",
      title: "Reopen Closed Tab",
      description: "Restore the most recently closed document",
      category: "File",
      enabled: canReopenClosedDocument && !operationInProgress,
      run: () => onAction("reopenClosed"),
    },
    {
      id: "file.clearRecentDocuments",
      title: "Clear Recent Documents",
      description: "Remove the recent document history",
      category: "File",
      enabled: canReopenClosedDocument && !operationInProgress,
      run: () => onAction("clearRecentDocuments"),
    },
    {
      id: "workbench.settings",
      title: "Open Settings",
      description: "Configure editor, shell, and theme preferences",
      category: "Preferences",
      shortcut: "Ctrl+,",
      enabled: !operationInProgress,
      run: openSettingsTab,
    },
    {
      id: "search.open",
      title: "Open Search",
      description: "Open search in the current workspace",
      category: "Search",
      enabled: !operationInProgress,
      run: () => setWorkspacePanel("search"),
    },
    {
      id: "workbench.notebookPanel",
      title: "Open Notebook Panel",
      description: "Show the Notebook tool panel shell",
      category: "Workbench",
      enabled: !operationInProgress,
      run: () => setWorkspacePanel("notebook"),
    },
    {
      id: "workbench.toggleAgentPanel",
      title: "Toggle Agent Panel",
      description: "Show or hide the Agent workspace panel",
      category: "Workbench",
      enabled: !operationInProgress,
      run: () => setAgentPanelOpen((open) => !open),
    },
  ];

  useEffect(() => {
    function handleShortcut(event: KeyboardEvent) {
      const modifier = event.ctrlKey || event.metaKey;
      if (modifier && event.shiftKey && event.key.toLowerCase() === "p") {
        event.preventDefault();
        openCommandBar();
        return;
      }
      if (event.key === "Escape" && commandBarOpen) {
        closeCommandBar();
        return;
      }
      if (!modifier || event.altKey || commandBarOpen || settingsTabActive || operationInProgress) return;
      const key = event.key.toLowerCase();
      if (key === "n" && !event.shiftKey) {
        event.preventDefault();
        onAction("new");
      } else if (key === "o" && !event.shiftKey) {
        event.preventDefault();
        onAction("open");
      } else if (key === "s" && event.shiftKey) {
        event.preventDefault();
        if (!showWelcome) onAction("saveAs");
      } else if (key === "s") {
        event.preventDefault();
        if (!showWelcome) onAction("save");
      } else if (key === "," && !event.shiftKey) {
        event.preventDefault();
        openSettingsTab();
      }
    }
    document.addEventListener("keydown", handleShortcut);
    return () => document.removeEventListener("keydown", handleShortcut);
  }, [commandBarOpen, onAction, operationInProgress, settingsTabActive, showWelcome]);

  useEffect(() => {
    if (!fileMenuOpen) {
      return;
    }
    const menu = fileMenuRef.current?.querySelector<HTMLElement>('[role="menu"]');
    const menuItems = () =>
      [...(menu?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? [])];
    menuItems()[0]?.focus();

    function handlePointerDown(event: PointerEvent) {
      if (event.target instanceof Node && !fileMenuRef.current?.contains(event.target)) {
        setFileMenuOpen(false);
      }
    }

    function handleFocusIn(event: FocusEvent) {
      if (event.target instanceof Node && !fileMenuRef.current?.contains(event.target)) {
        setFileMenuOpen(false);
      }
    }

    function handleMenuKeyDown(event: KeyboardEvent) {
      const items = menuItems();
      const selectedIndex = items.indexOf(document.activeElement as HTMLButtonElement);
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const direction = event.key === "ArrowDown" ? 1 : -1;
        const nextIndex = selectedIndex < 0
          ? direction > 0 ? 0 : items.length - 1
          : (selectedIndex + direction + items.length) % items.length;
        items[nextIndex]?.focus();
      } else if (event.key === "Home" || event.key === "End") {
        event.preventDefault();
        items[event.key === "Home" ? 0 : items.length - 1]?.focus();
      } else if (event.key === "Escape") {
        event.preventDefault();
        setFileMenuOpen(false);
        fileMenuTriggerRef.current?.focus();
      }
    }

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("focusin", handleFocusIn);
    document.addEventListener("keydown", handleMenuKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("focusin", handleFocusIn);
      document.removeEventListener("keydown", handleMenuKeyDown);
    };
  }, [fileMenuOpen]);

  const languageName = languageOptions.find((option) => option.id === activeDocument?.language)?.label
    ?? languageLabel(activeDocument?.language);
  const languageCommands: ShellCommand[] = languageOptions.map((option) => ({
    id: `language.set.${option.id}`,
    title: option.label,
    description: `Set the language mode to ${option.label}`,
    category: "Language Mode",
    enabled: activeDocument !== null && !operationInProgress,
    run: () => {
      if (activeDocument) {
        void onChangeLanguage(activeDocument.documentId, option.id);
      }
    },
  }));
  const welcomePanelVisible = showWelcome && !settingsTabOpen;

  return (
    <main
      onClick={() => setContextMenu(null)}
      onContextMenu={handleAppContextMenu}
      onKeyDown={handleAppKeyDown}
    >
      <header className="app-header">
        <nav className="menubar" aria-label="Application menu">
          <div className="file-menu-container" ref={fileMenuRef}>
            <button
              aria-controls="file-menu"
              aria-expanded={fileMenuOpen}
              aria-haspopup="menu"
              className="menubar-button"
              disabled={operationInProgress}
              onClick={() => setFileMenuOpen((open) => !open)}
              ref={fileMenuTriggerRef}
              type="button"
            >
              File
            </button>
            <div
              aria-label="File"
              className="file-menu-popup"
              hidden={!fileMenuOpen}
              id="file-menu"
              role="menu"
            >
              <button
                disabled={operationInProgress}
                onClick={() => runFileAction("openWorkspace")}
                role="menuitem"
                type="button"
              >
                Open Folder
              </button>
              <button
                disabled={!workspace || operationInProgress}
                onClick={() => runFileAction("closeWorkspace")}
                role="menuitem"
                type="button"
              >
                Close Folder
              </button>
              <div className="file-menu-separator" role="separator" />
              <button
                disabled={operationInProgress}
                onClick={() => runFileAction("new")}
                role="menuitem"
                type="button"
              >
                New
              </button>
              <button
                disabled={operationInProgress}
                onClick={() => runFileAction("open")}
                role="menuitem"
                type="button"
              >
                Open
              </button>
              <div className="file-menu-separator" role="separator" />
              <button
                disabled={showWelcome || operationInProgress}
                onClick={() => runFileAction("save")}
                role="menuitem"
                type="button"
              >
                Save
              </button>
              <button
                disabled={showWelcome || operationInProgress}
                onClick={() => runFileAction("saveAs")}
                role="menuitem"
                type="button"
              >
                Save As
              </button>
              <div className="file-menu-separator" role="separator" />
              <button
                disabled={operationInProgress}
                onClick={() => runFileAction("exit")}
                role="menuitem"
                type="button"
              >
                Exit
              </button>
            </div>
          </div>
          <button
            aria-keyshortcuts="Control+Comma Meta+Comma"
            className="menubar-button"
            disabled={operationInProgress}
            onClick={openSettingsTab}
            type="button"
          >
            Settings
          </button>
          <button
            aria-expanded={agentPanelOpen}
            aria-label="Toggle Agent panel"
            className="menubar-button"
            disabled={operationInProgress}
            onClick={() => setAgentPanelOpen((open) => !open)}
            type="button"
          >
            Agent
          </button>
        </nav>
        <CommandBar
          commands={commandBarPurpose === "languages" ? languageCommands : commands}
          isOpen={commandBarOpen}
          onOpen={() => openCommandBar()}
          onClose={closeCommandBar}
          purpose={commandBarPurpose}
        />
      </header>
      <div className="workspace-layout">
        <div className="workspace-sidebar">
          <nav className="workspace-tools" role="tablist" aria-label="Workspace panels">
            <button
              aria-selected={workspacePanel === "files"}
              onClick={() => setWorkspacePanel("files")}
              role="tab"
              type="button"
            >
              Files
            </button>
            <button
              aria-selected={workspacePanel === "search"}
              onClick={() => setWorkspacePanel("search")}
              role="tab"
              type="button"
            >
              Search
            </button>
            <button
              aria-selected={workspacePanel === "notebook"}
              onClick={() => setWorkspacePanel("notebook")}
              role="tab"
              type="button"
            >
              Notebook
            </button>
          </nav>
          {workspacePanel === "files" ? (
            <WorkspaceExplorer
              workspace={workspace}
              workspaceError={workspaceError}
              operationInProgress={operationInProgress}
              onLoadDirectory={onLoadWorkspaceDirectory}
              onOpenFile={onOpenWorkspaceFile}
              contextMenu={contextMenu}
              onContextMenuEntry={(event, entry) => {
                contextMenuTriggerRef.current = event.currentTarget;
                showWorkspaceEntryContextMenu(event, entry);
              }}
              onDismissContextMenu={() => setContextMenu(null)}
            />
          ) : workspacePanel === "search" ? (
            <WorkspaceSearch
              workspace={workspace}
              documents={documents}
              operationInProgress={operationInProgress}
              onSearch={onSearchWorkspace}
              onCancel={onCancelSearch}
              onOpenResult={onOpenWorkspaceSearchResult}
              onSetHighlights={onSetSearchHighlights}
              onReplace={onReplaceSearchResults}
            />
          ) : (
            <aside className="workspace-notebook" aria-label="Notebook tools">
              <h2>Notebook</h2>
              <p className="workspace-empty">
                The Notebook panel shell is available. Notebook execution is not implemented.
              </p>
            </aside>
          )}
        </div>
        <section className="editor-panel" aria-label="Editor">
          {welcomePanelVisible ? (
            <div className="welcome-panel" aria-labelledby="welcome-title">
              <p className="eyebrow">Workspace</p>
              <h2 id="welcome-title">Welcome</h2>
              <p>Open a project folder to browse its files, or open a file to begin editing.</p>
            </div>
          ) : (
            <>
              <DocumentTabs
                documents={showWelcome ? [] : documents}
                activeViewId={showWelcome ? null : activeViewId}
                onSwitchDocument={(viewId) => {
                  setSettingsTabActive(false);
                  onSwitchDocument(viewId);
                }}
                onRequestClose={(document) => void requestClose(document)}
                operationInProgress={operationInProgress}
                settingsTabOpen={settingsTabOpen}
                settingsTabActive={settingsTabActive}
                settingsBusy={settingsBusy}
                onActivateSettings={() => setSettingsTabActive(true)}
                onCloseSettings={closeSettingsTab}
              />
            </>
          )}
          {settingsTabOpen && (
            <div
              aria-labelledby="settings-tab"
              className="settings-tab-content"
              hidden={!settingsTabActive}
              id="settings-tabpanel"
              role="tabpanel"
            >
              <SettingsDialog
                onBusyChange={setSettingsBusy}
                onLoadSettings={onLoadSettings}
                onApplySettings={onApplySettings}
                onLoadUserTheme={onLoadUserTheme}
                onSaveUserTheme={onSaveUserTheme}
              />
            </div>
          )}
          <div
            ref={editorRef}
            hidden={welcomePanelVisible || settingsTabActive}
            id="editor"
            data-context-menu-surface="editor"
            role="region"
            aria-label="Code editor"
            onContextMenu={showEditorContextMenu}
          />
          {!welcomePanelVisible && !settingsTabActive && (
            <div className="editor-toolbar">
              <span className="file-name">{activeTitle}</span>
              <div className="editor-toolbar-meta">
                <button
                  aria-expanded={commandBarOpen && commandBarPurpose === "languages"}
                  aria-haspopup="dialog"
                  aria-label={`Change language mode, currently ${languageName}`}
                  className="language-mode-button"
                  disabled={!activeDocument || operationInProgress || languageOptions.length === 0}
                  onClick={() => openCommandBar("languages")}
                  type="button"
                >
                  {languageName}
                </button>
                <span className="editor-status" aria-live="polite">
                  {activeDocument && ` · Ln ${activeDocument.cursor.lineNumber}, Col ${activeDocument.cursor.column}`}
                  {activeDocument && ` · ${activeDocument.dirty ? "Modified" : "Saved"}`}
                  {documentStatus && ` · ${documentStatus}`}
                </span>
              </div>
            </div>
          )}
          {contextMenu?.kind === "editor" && (
            <div
              aria-label="Editor actions"
              className="context-menu"
              data-context-menu-surface="context-menu"
              ref={contextMenuRef}
              role="menu"
              style={{ left: contextMenu.left, top: contextMenu.top }}
              tabIndex={-1}
            >
              <button disabled role="menuitem" type="button">Send to Agent</button>
            </div>
          )}
        </section>
        {agentPanelOpen && (
          <aside className="agent-panel" aria-label="Agent panel">
            <h2>Agent</h2>
            <p className="workspace-empty">
              The Agent workspace panel is a shell. Agent integration is not implemented.
            </p>
          </aside>
        )}
      </div>
      <footer>
        <p role="status" aria-live="polite">{status}</p>
      </footer>
      {workspaceActionNeedsSave && (
        <div className="modal-backdrop">
          <section
            aria-labelledby="workspace-switch-title"
            aria-modal="true"
            className="confirmation-dialog"
            role="alertdialog"
          >
            <h2 id="workspace-switch-title">
              Save changes before {workspaceActionNeedsSave === "close" ? "closing the folder" : "opening a workspace"}?
            </h2>
            <p>
              {workspaceActionNeedsSave === "close"
                ? "Closing the folder closes the current document tabs. Save all unsaved changes to continue, or cancel to keep this workspace open."
                : "Opening a workspace closes the current document tabs. Save all unsaved changes to continue, or cancel to keep this workspace open."}
            </p>
            <ul aria-label="Documents with unsaved changes">
              {documents.filter((document) => document.dirty).map((document) => (
                <li key={document.documentId}>{document.title}</li>
              ))}
            </ul>
            <div className="confirmation-actions">
              <button
                disabled={operationInProgress}
                onClick={() => void saveAndContinueWorkspaceAction()}
                type="button"
              >
                Save All and {workspaceActionNeedsSave === "close" ? "Close Folder" : "Open Folder"}
              </button>
              <button
                className="secondary"
                disabled={operationInProgress}
                onClick={onDismissWorkspaceActionPrompt}
                type="button"
              >
                Cancel
              </button>
            </div>
          </section>
        </div>
      )}
      {pendingClose && (
        <div className="modal-backdrop">
          <section
            aria-labelledby="close-document-title"
            aria-modal="true"
            className="confirmation-dialog"
            role="alertdialog"
          >
            <h2 id="close-document-title">Save changes to {pendingClose.title}?</h2>
            <p>Your changes will be lost if you close this document without saving.</p>
            <div className="confirmation-actions">
              <button
                disabled={operationInProgress}
                onClick={() => void closeAfterSave()}
                type="button"
              >
                Save
              </button>
              <button
                className="secondary"
                disabled={operationInProgress}
                onClick={() => void discardAndClose()}
                type="button"
              >
                Discard Changes
              </button>
              <button
                className="secondary"
                disabled={operationInProgress}
                onClick={() => setPendingClose(null)}
                type="button"
              >
                Cancel
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}

import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { resolve } from "node:path";
import { test } from "node:test";
import { createServer } from "vite";

const webRoot = fileURLToPath(new URL("..", import.meta.url));

test("workspace shell renders document actions, tabs, editor, and status accessibly", async () => {
  const server = await createServer({
    configFile: resolve(webRoot, "vite.config.js"),
    server: { middlewareMode: true, hmr: false },
    appType: "custom",
  });

  try {
    const { WorkspaceShell, languageLabel } = await server.ssrLoadModule(
      resolve(webRoot, "../functor_ui/src/index.tsx"),
    );
    assert.deepEqual(
      ["fsharp", "csharp", "json", "markdown", "plainText"].map(languageLabel),
      ["F#", "C#", "JSON", "Markdown", "Plain Text"],
    );
    const html = renderToStaticMarkup(createElement(WorkspaceShell, {
      documents: [
        { documentId: 2, viewId: 2, title: "notes.md", dirty: true },
        { documentId: 3, viewId: 3, title: "main.rs", dirty: false },
      ],
      activeViewId: 2,
      activeTitle: "notes.md",
      activeDocument: {
        documentId: 2,
        language: "markdown",
        cursor: { lineNumber: 7, column: 11 },
        dirty: true,
      },
      languageOptions: [
        { id: "plainText", label: "Plain Text" },
        { id: "markdown", label: "Markdown" },
        { id: "typescript", label: "TypeScript" },
      ],
      canReopenClosedDocument: true,
      workspace: null,
      workspaceError: null,
      status: "Document opened.",
      documentStatus: "Unsaved changes",
      operationInProgress: false,
      workspaceActionNeedsSave: "open",
      editorRef: { current: null },
      onAction() {},
      onSwitchDocument() {},
      onChangeLanguage: async () => true,
      onSaveDocument: async () => true,
      onSaveAllDocuments: async () => true,
      onDismissWorkspaceActionPrompt() {},
      onCloseDocument: async () => true,
      onSearchWorkspace: async () => ({ hits: [], skippedFiles: 0, warnings: [] }),
      onOpenWorkspaceSearchResult: async () => true,
      onLoadWorkspaceDirectory: async () => true,
      onOpenWorkspaceFile: async () => true,
    }));

    assert.match(html, /aria-label="Application menu"/);
    assert.match(html, /role="combobox"/);
    assert.match(html, /aria-label="Search or run a command"/);
    assert.doesNotMatch(html, /Folder-based project workspace|<h1>Functor<\/h1>/);
    assert.match(html, /aria-haspopup="menu"/);
    assert.match(html, /aria-expanded="false"[^>]*>File<\/button>/);
    assert.match(html, /aria-label="File"[^>]*hidden=""/);
    assert.match(html, /role="menu"/);
    assert.match(html, />Open Folder</);
    assert.match(html, />Close Folder<\/button>/);
    assert.match(html, />New</);
    assert.match(html, />Open</);
    assert.match(html, />Save</);
    assert.match(html, />Save As</);
    assert.match(html, />Exit</);
    assert.match(html, />Settings</);
    assert.doesNotMatch(html, />Commands</);
    assert.doesNotMatch(html, /Reopen Closed/);
    assert.doesNotMatch(html, /revision 4/);
    assert.match(html, /aria-label="Project files"/);
    assert.match(html, /Open a folder to browse and organize project files\./);
    assert.match(html, /role="tab" aria-selected="true"[^>]*>● notes\.md</);
    assert.match(html, /role="tab" aria-selected="false"[^>]*>main\.rs</);
    assert.match(html, /aria-label="Close notes\.md"/);
    assert.match(html, /aria-label="Workspace panels"/);
    assert.match(html, />Search<\/button>/);
    assert.match(html, />Notebook<\/button>/);
    assert.match(html, /aria-label="Toggle Agent panel"/);
    assert.ok(html.indexOf(">Settings</button>") < html.indexOf('aria-label="Toggle Agent panel"'));
    assert.ok(
      html.indexOf('aria-label="Toggle Agent panel"')
        < html.indexOf('class="command-bar"'),
    );
    assert.ok(
      html.indexOf('aria-label="Toggle Agent panel"')
        < html.indexOf('class="workspace-layout"'),
    );
    assert.match(
      html,
      /aria-label="Change language mode, currently Markdown"[^>]*>Markdown<\/button>/,
    );
    assert.doesNotMatch(html, /<select[^>]*aria-label="Language mode"/);
    assert.match(html, /id="editor"[^>]*role="region" aria-label="Code editor"/);
    assert.match(html, /data-context-menu-surface="editor"/);
    assert.ok(html.indexOf('id="editor"') < html.indexOf('class="editor-toolbar"'));
    assert.match(html, /role="status" aria-live="polite">Document opened\.<\/p>/);
    assert.match(html, /Unsaved changes/);
    assert.match(html, /Ln 7, Col 11 · Modified · Unsaved changes/);
    assert.doesNotMatch(html, /recovery (saved|pending)/i);
    assert.doesNotMatch(html, /Rust MVU owns project state/);
    assert.match(html, /Save changes before opening a workspace\?/);
    assert.match(html, /Save All and Open Folder/);
    assert.match(html, /Documents with unsaved changes/);
    assert.doesNotMatch(html, /Discard Changes/);
  } finally {
    await server.close();
  }
});

test("workspace explorer renders folders and accessible file actions", async () => {
  const server = await createServer({
    configFile: resolve(webRoot, "vite.config.js"),
    server: { middlewareMode: true, hmr: false },
    appType: "custom",
  });

  try {
    const { WorkspaceShell } = await server.ssrLoadModule(
      resolve(webRoot, "../functor_ui/src/index.tsx"),
    );
    const html = renderToStaticMarkup(createElement(WorkspaceShell, {
      documents: [],
      activeViewId: null,
      activeTitle: "Loading...",
      workspace: {
        root: "C:\\project",
        name: "project",
        entries: [
          { name: "src", relativePath: "src", kind: "directory", children: null },
          { name: "README.md", relativePath: "README.md", kind: "file", children: null },
        ],
      },
      workspaceError: null,
      status: "Workspace opened.",
      documentStatus: "",
      operationInProgress: false,
      workspaceActionNeedsSave: null,
      editorRef: { current: null },
      onAction() {},
      onSwitchDocument() {},
      onSaveDocument: async () => true,
      onSaveAllDocuments: async () => true,
      onDismissWorkspaceActionPrompt() {},
      onCloseDocument: async () => true,
      onSearchWorkspace: async () => ({ hits: [], skippedFiles: 0, warnings: [] }),
      onOpenWorkspaceSearchResult: async () => true,
      onLoadWorkspaceDirectory: async () => true,
      onOpenWorkspaceFile: async () => true,
    }));

    assert.match(html, /aria-label="project files"/);
    assert.match(html, /aria-label="Expand src"/);
    assert.match(html, /data-context-menu-surface="workspace-entry"/);
    assert.match(html, /aria-label="Open README\.md"/);
    assert.match(html, />Close Folder</);
  } finally {
    await server.close();
  }
});

test("workspace restore errors remain visible before a project is opened", async () => {
  const server = await createServer({
    configFile: resolve(webRoot, "vite.config.js"),
    server: { middlewareMode: true, hmr: false },
    appType: "custom",
  });

  try {
    const { WorkspaceShell } = await server.ssrLoadModule(
      resolve(webRoot, "../functor_ui/src/index.tsx"),
    );
    const html = renderToStaticMarkup(createElement(WorkspaceShell, {
      documents: [],
      activeViewId: null,
      activeTitle: "Loading...",
      workspace: null,
      workspaceError: "The saved project folder is unavailable.",
      status: "Workspace restore failed.",
      documentStatus: "",
      operationInProgress: false,
      workspaceActionNeedsSave: null,
      editorRef: { current: null },
      onAction() {},
      onSwitchDocument() {},
      onSaveDocument: async () => true,
      onSaveAllDocuments: async () => true,
      onDismissWorkspaceActionPrompt() {},
      onCloseDocument: async () => true,
      onSearchWorkspace: async () => ({ hits: [], skippedFiles: 0, warnings: [] }),
      onOpenWorkspaceSearchResult: async () => true,
      onLoadWorkspaceDirectory: async () => false,
      onOpenWorkspaceFile: async () => false,
    }));

    assert.match(html, /role="alert">The saved project folder is unavailable\.<\/p>/);
    assert.match(html, /Open a folder to browse and organize project files\./);
  } finally {
    await server.close();
  }
});

test("recovery snapshots no longer block the workspace shell with a prompt", async () => {
  const server = await createServer({
    configFile: resolve(webRoot, "vite.config.js"),
    server: { middlewareMode: true, hmr: false },
    appType: "custom",
  });

  try {
    const { WorkspaceShell } = await server.ssrLoadModule(
      resolve(webRoot, "../functor_ui/src/index.tsx"),
    );
    const html = renderToStaticMarkup(createElement(WorkspaceShell, {
      documents: [],
      activeViewId: null,
      activeTitle: "Loading...",
      workspace: null,
      workspaceError: null,
      recoveryCandidates: [
        { recoveryId: "file-01", title: "main.rs", path: "C:\\project\\src\\main.rs" },
      ],
      status: "Ready.",
      documentStatus: "",
      operationInProgress: false,
      workspaceActionNeedsSave: null,
      editorRef: { current: null },
      onAction() {},
      onSwitchDocument() {},
      onSaveDocument: async () => true,
      onSaveAllDocuments: async () => true,
      onDismissWorkspaceActionPrompt() {},
      onCloseDocument: async () => true,
      onSearchWorkspace: async () => ({ hits: [], skippedFiles: 0, warnings: [] }),
      onOpenWorkspaceSearchResult: async () => true,
      onLoadWorkspaceDirectory: async () => false,
      onOpenWorkspaceFile: async () => false,
    }));

    assert.doesNotMatch(html, /Recover unsaved work\?/);
    assert.doesNotMatch(html, /Restore Unsaved Work/);
    assert.doesNotMatch(html, /Discard Recovery Data/);
  } finally {
    await server.close();
  }
});

test("welcome panel offers workspace and file actions instead of an empty editor", async () => {
  const server = await createServer({
    configFile: resolve(webRoot, "vite.config.js"),
    server: { middlewareMode: true, hmr: false },
    appType: "custom",
  });

  try {
    const { WorkspaceShell } = await server.ssrLoadModule(
      resolve(webRoot, "../functor_ui/src/index.tsx"),
    );
    const html = renderToStaticMarkup(createElement(WorkspaceShell, {
      documents: [{ documentId: 1, viewId: 1, title: "Untitled 1", dirty: false }],
      activeViewId: 1,
      activeTitle: "Untitled 1",
      activeDocument: null,
      languageOptions: [],
      canReopenClosedDocument: false,
      showWelcome: true,
      workspace: null,
      workspaceError: null,
      status: "Ready.",
      documentStatus: "",
      operationInProgress: false,
      workspaceActionNeedsSave: null,
      editorRef: { current: null },
      onAction() {},
      onSwitchDocument() {},
      onChangeLanguage: async () => true,
      onSaveDocument: async () => true,
      onSaveAllDocuments: async () => true,
      onDismissWorkspaceActionPrompt() {},
      onCloseDocument: async () => true,
      onSearchWorkspace: async () => ({ hits: [], skippedFiles: 0, warnings: [] }),
      onCancelSearch: async () => true,
      onOpenWorkspaceSearchResult: async () => true,
      onSetSearchHighlights() {},
      onReplaceSearchResults: async () => ({ replaced: 0 }),
      onLoadWorkspaceDirectory: async () => true,
      onOpenWorkspaceFile: async () => true,
      onLoadSettings: async () => ({ settings: {}, presets: [], userThemes: [], warnings: [] }),
      onApplySettings: async (settings) => settings,
      onLoadUserTheme: async () => ({}),
      onSaveUserTheme: async () => ({ name: "theme", preset: {} }),
    }));

    assert.match(html, /<h2 id="welcome-title">Welcome<\/h2>/);
    assert.match(html, />Open Folder</);
    assert.doesNotMatch(html, />Open Workspace</);
    assert.match(html, />Open<\/button>/);
    assert.match(html, /aria-label="Toggle Agent panel"/);
    assert.match(html, /<button disabled=""[^>]*>Save<\/button>/);
    assert.match(html, /<button disabled=""[^>]*>Save As<\/button>/);
    assert.doesNotMatch(html, /aria-label="Open documents"/);
    assert.match(html, /hidden="" id="editor"[^>]*role="region" aria-label="Code editor"/);
  } finally {
    await server.close();
  }
});

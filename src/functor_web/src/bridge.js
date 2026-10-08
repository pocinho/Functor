import * as monaco from "monaco-editor/editor/editor.api.js";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import "monaco-editor/editor/contrib/find/browser/findController.js";
import "monaco-editor/languages/definitions/register.all.js";
import "../node_modules/monaco-editor/min/vs/editor/editor.main.css";
import {
  graphemeBoundaries,
  nextGraphemeBoundary,
  previousGraphemeBoundary,
} from "./grapheme_boundaries.js";
import { mapMonacoSelection } from "./monaco_selection.js";
import { toOpenWorkspaceSearchResultRequest } from "./search_result_request.js";
import { planOpenSearchReplacements } from "./search_replacement_plan.js";
import { applyEditorSettings } from "./settings_theme.js";
import { registerJsonLanguage } from "./json_language.js";
import { getMonacoLanguageOptions, toMonacoLanguage } from "./monaco_language.js";
import { restoreStartupDocuments } from "./startup_restore.js";

globalThis.MonacoEnvironment = {
  getWorker() {
    return new EditorWorker();
  },
};
registerJsonLanguage(monaco);
const languageOptions = getMonacoLanguageOptions(monaco.languages.getLanguages());
const registeredLanguageIds = new Set(languageOptions.map(({ id }) => id));

const invoke = window.__TAURI__?.core?.invoke;
let editor;
let records;
let documentQueues;
let recoveryTimers;
let recoveryWrites;
let suppressedDocuments;
let modelDocumentIds;
let callbacks;
let activeDocumentId = null;
let currentSnapshot = null;
let operationInProgress = false;
let disposed = false;
let workspacePersistenceWarning = null;
let settingsLoadWarning = null;
let settingsRequestVersion = 0;
const graphemeBoundaryCache = new WeakMap();

function notify(callback, value) {
  if (!disposed) {
    if (callback === "onStatus" && workspacePersistenceWarning) {
      value = `${value} ${workspacePersistenceWarning}`;
    }
    if (callback === "onStatus" && settingsLoadWarning) {
      value = `${value} ${settingsLoadWarning}`;
    }
    callbacks[callback](value);
  }
}

function bridgeErrorMessage(error) {
  if (error && typeof error === "object" && "message" in error) {
    return error.message;
  }
  return String(error);
}

function bridgeErrorCode(error) {
  if (error && typeof error === "object" && "code" in error) {
    return error.code;
  }
  return null;
}

function bridgeInvoke(command, args = {}) {
  if (typeof invoke !== "function") {
    return Promise.reject(new Error("The Tauri command bridge is unavailable."));
  }
  return invoke(command, args);
}

function ensureRecord(snapshot) {
  const id = String(snapshot.documentId);
  let record = records.get(id);
  if (!record) {
    const model = monaco.editor.createModel(
      snapshot.text,
      toMonacoLanguage(snapshot.language, registeredLanguageIds),
      monaco.Uri.parse(`inmemory://functors/${snapshot.viewId}/${encodeURIComponent(snapshot.title)}`),
    );
    record = {
      id,
      model,
      revision: snapshot.revision,
      snapshot,
      syncedText: snapshot.text,
      syncError: null,
      recoverySavedRevision: null,
      searchDecorations: [],
      viewState: null,
      viewStateText: null,
      viewStateCursor: null,
      viewStateSelection: null,
    };
    records.set(id, record);
    modelDocumentIds.set(model, id);
    attachModelHandlers(record);
  } else {
    const revisionChanged = record.revision !== snapshot.revision;
    record.revision = snapshot.revision;
    record.snapshot = snapshot;
    record.syncError = null;
    if (revisionChanged || !snapshot.dirty) {
      record.recoverySavedRevision = null;
    }
    const language = toMonacoLanguage(snapshot.language, registeredLanguageIds);
    if (record.model.getLanguageId() !== language) {
      monaco.editor.setModelLanguage(record.model, language);
    }
    if (record.model.getValue() !== snapshot.text) {
      suppressedDocuments.add(id);
      record.model.setValue(snapshot.text);
      suppressedDocuments.delete(id);
    }
    record.syncedText = snapshot.text;
  }
  return record;
}

function cursorSelection(model = editor.getModel(), position = editor.getPosition(), selection = editor.getSelection()) {
  return mapMonacoSelection(model, position, selection);
}

function offsetForPosition(text, position) {
  const lines = text.split("\n");
  const targetLine = Math.min(Math.max(position.lineNumber - 1, 0), lines.length - 1);
  let offset = 0;
  for (let index = 0; index < targetLine; index += 1) {
    offset += lines[index].length + 1;
  }
  return offset + Math.min(Math.max(position.column - 1, 0), lines[targetLine].length);
}

function transformOffset(offset, changes) {
  let adjustment = 0;
  for (const change of [...changes].sort((left, right) => left.rangeOffset - right.rangeOffset)) {
    const start = change.rangeOffset;
    const end = start + change.rangeLength;
    if (offset < start) {
      break;
    }
    if (offset <= end) {
      return start + adjustment + change.text.length;
    }
    adjustment += change.text.length - change.rangeLength;
  }
  return offset + adjustment;
}

function selectionAfterModelChange(record, event) {
  if (modelDocumentIds.get(editor.getModel()) === record.id) {
    return cursorSelection(record.model);
  }
  const positionAfterChange = (position) => record.model.getPositionAt(
    transformOffset(
      offsetForPosition(record.snapshot.text, position),
      event.changes,
    ),
  );
  return {
    cursor: positionAfterChange(record.snapshot.cursor),
    selection: record.snapshot.selection
      ? {
          anchor: positionAfterChange(record.snapshot.selection.anchor),
          focus: positionAfterChange(record.snapshot.selection.focus),
        }
      : null,
  };
}

function enqueueDocumentOperation(record, operation) {
  const previous = documentQueues.get(record.id) ?? Promise.resolve();
  const next = previous.then(async () => {
    if (record.syncError) {
      throw record.syncError;
    }
    await operation();
  }).catch((error) => {
    if (disposed) {
      return;
    }
    record.syncError = error;
    if (modelDocumentIds.get(editor.getModel()) === record.id) {
      editor.updateOptions({ readOnly: true });
    }
    notify("onStatus", `Editor synchronization failed: ${bridgeErrorMessage(error)}`);
    updateDocumentStatus(record.snapshot);
  });
  documentQueues.set(record.id, next);
  return next;
}

function attachModelHandlers(record) {
  record.model.onDidChangeContent((event) => {
    if (suppressedDocuments.has(record.id)) {
      return;
    }
    if (record.searchDecorations.length > 0) {
      record.searchDecorations = record.model.deltaDecorations(record.searchDecorations, []);
    }
    const { cursor, selection } = selectionAfterModelChange(record, event);
    const changes = event.changes.map((change) => ({
      rangeOffset: change.rangeOffset,
      rangeLength: change.rangeLength,
      text: change.text,
    }));
    enqueueDocumentOperation(record, async () => {
      const response = await bridgeInvoke("apply_document_edits", {
        request: {
          documentId: Number(record.id),
          expectedRevision: record.revision,
          changes,
          isUndoing: event.isUndoing,
          isRedoing: event.isRedoing,
          cursor,
          selection,
        },
      });
      record.revision = response.revision;
      record.syncedText = record.model.getValue();
      record.snapshot = {
        ...record.snapshot,
        revision: response.revision,
        dirty: response.dirty,
        text: record.syncedText,
        cursor,
        selection,
      };
      publishDocumentSnapshot(record);
      updateDocumentStatus(record.snapshot);
      scheduleRecoverySnapshot(record);
    });
  });
}

function attachCursorSelectionHandler() {
  editor.onDidChangeCursorSelection((event) => {
    const id = modelDocumentIds.get(editor.getModel());
    const record = id && records.get(id);
    if (!record || suppressedDocuments.has(id)) {
      return;
    }
    const { cursor, selection } = cursorSelection(editor.getModel(), event.position, event.selection);
    // Monaco can publish the post-edit selection before its content event on redo.
    queueMicrotask(() => {
      if (disposed || records.get(id) !== record) {
        return;
      }
      enqueueDocumentOperation(record, async () => {
        const response = await bridgeInvoke("update_selection", {
          request: {
            documentId: Number(record.id),
            expectedRevision: record.revision,
            cursor,
            selection,
          },
        });
        record.revision = response.revision;
        record.snapshot = {
          ...record.snapshot,
          cursor,
          selection,
        };
        publishDocumentSnapshot(record);
      });
    });
  });
}

function attachGraphemeInputHandler() {
  editor.onKeyDown((event) => {
    const browserEvent = event.browserEvent;
    const key = browserEvent.key;
    if (
      browserEvent.isComposing
      || browserEvent.ctrlKey
      || browserEvent.metaKey
      || browserEvent.altKey
      || !["ArrowLeft", "ArrowRight", "Backspace", "Delete"].includes(key)
    ) {
      return;
    }

    const model = editor.getModel();
    const selection = editor.getSelection();
    const isArrow = key === "ArrowLeft" || key === "ArrowRight";
    if (
      !model
      || !selection
      || (!selection.isEmpty() && (!isArrow || !browserEvent.shiftKey))
    ) {
      return;
    }

    const position = selection.getPosition();
    const offset = position.column - 1;
    const boundaries = lineGraphemeBoundaries(model, position.lineNumber);
    const previous = previousGraphemeBoundary(boundaries, offset);
    const next = nextGraphemeBoundary(boundaries, offset);

    if (isArrow) {
      const target = key === "ArrowLeft" ? previous : next;
      if (target === null) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      const targetPosition = new monaco.Position(position.lineNumber, target + 1);
      if (browserEvent.shiftKey) {
        editor.setSelection(new monaco.Selection(
          selection.selectionStartLineNumber,
          selection.selectionStartColumn,
          targetPosition.lineNumber,
          targetPosition.column,
        ));
      } else {
        editor.setPosition(targetPosition);
      }
      return;
    }

    const start = key === "Backspace"
      ? previous
      : boundaries.includes(offset) ? offset : previous;
    const end = key === "Delete"
      ? next
      : boundaries.includes(offset) ? offset : next;
    if (start === null || end === null || start === end) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const cursor = new monaco.Position(position.lineNumber, start + 1);
    editor.executeEdits(
      "functor.grapheme-delete",
      [{
        range: new monaco.Range(
          position.lineNumber,
          start + 1,
          position.lineNumber,
          end + 1,
        ),
        text: "",
      }],
      () => [new monaco.Selection(cursor.lineNumber, cursor.column, cursor.lineNumber, cursor.column)],
    );
  });
}

function lineGraphemeBoundaries(model, lineNumber) {
  const version = model.getVersionId();
  let cache = graphemeBoundaryCache.get(model);
  if (!cache || cache.version !== version) {
    cache = { version, lines: new Map() };
    graphemeBoundaryCache.set(model, cache);
  }
  let boundaries = cache.lines.get(lineNumber);
  if (!boundaries) {
    boundaries = graphemeBoundaries(model.getLineContent(lineNumber));
    cache.lines.set(lineNumber, boundaries);
  }
  return boundaries;
}

function publishDocumentSnapshot(record) {
  if (!currentSnapshot) {
    return;
  }
  currentSnapshot = {
    ...currentSnapshot,
    documents: currentSnapshot.documents.map((document) =>
      document.documentId === record.snapshot.documentId
        ? record.snapshot
        : document),
  };
  notify("onSnapshot", currentSnapshot);
}

function updateDocumentStatus(snapshot) {
  if (!snapshot) {
    notify("onDocumentStatus", "");
    return;
  }
  notify("onDocumentStatus", snapshot.dirty ? "Unsaved changes" : "Saved");
}

function clearRecoveryTimer(documentId) {
  const timers = recoveryTimers.get(documentId);
  if (!timers) {
    return;
  }
  clearTimeout(timers.quiet);
  clearTimeout(timers.maximum);
  recoveryTimers.delete(documentId);
}

function writeRecoverySnapshot(record) {
  const previous = recoveryWrites.get(record.id) ?? Promise.resolve();
  const write = previous.catch(() => undefined).then(async () => {
    if (disposed || records.get(record.id) !== record) {
      return;
    }
    const response = await bridgeInvoke("save_recovery_snapshot", {
      request: {
        documentId: Number(record.id),
        expectedRevision: record.revision,
      },
    });
    if (response.stale) {
      return;
    }
    record.recoverySavedRevision = response.savedRevision;
    updateDocumentStatus(record.snapshot);
  });
  recoveryWrites.set(record.id, write);
  const clearWrite = () => {
    if (recoveryWrites.get(record.id) === write) {
      recoveryWrites.delete(record.id);
    }
  };
  write.then(clearWrite, clearWrite);
  write.catch((error) => {
    notify("onStatus", `Recovery autosave failed: ${bridgeErrorMessage(error)}`);
  });
  return write;
}

function scheduleRecoverySnapshot(record) {
  let timers = recoveryTimers.get(record.id);
  if (!timers) {
    timers = {
      quiet: null,
      maximum: setTimeout(() => {
        clearRecoveryTimer(record.id);
        void writeRecoverySnapshot(record);
      }, 2000),
    };
    recoveryTimers.set(record.id, timers);
  } else {
    clearTimeout(timers.quiet);
  }
  timers.quiet = setTimeout(() => {
    clearRecoveryTimer(record.id);
    void writeRecoverySnapshot(record);
  }, 500);
}

async function flushRecoveryWrites() {
  for (const documentId of [...recoveryTimers.keys()]) {
    clearRecoveryTimer(documentId);
  }
  await Promise.all([...recoveryWrites.values()].map((write) => write.catch(() => undefined)));
  const writes = [...records.values()].map(writeRecoverySnapshot);
  await Promise.all(writes.map((write) => write.catch(() => undefined)));
}

function renderSnapshot(snapshot, focusEditor = false) {
  if (disposed) {
    return;
  }
  workspacePersistenceWarning = snapshot.persistenceWarning;
  const activeSnapshot = snapshot.documents.find(
    (entry) => entry.viewId === snapshot.activeViewId,
  );
  if (!activeSnapshot) {
    throw new Error("The Rust snapshot has no active document.");
  }
  const nextActiveId = String(activeSnapshot.documentId);
  const previousActiveRecord = records.get(activeDocumentId);
  if (
    previousActiveRecord
    && previousActiveRecord.id !== nextActiveId
    && editor.getModel() === previousActiveRecord.model
  ) {
    previousActiveRecord.viewState = editor.saveViewState();
    previousActiveRecord.viewStateText = previousActiveRecord.snapshot.text;
    previousActiveRecord.viewStateCursor = previousActiveRecord.snapshot.cursor;
    previousActiveRecord.viewStateSelection = previousActiveRecord.snapshot.selection;
  }
  currentSnapshot = snapshot;

  for (const documentSnapshot of snapshot.documents) {
    ensureRecord(documentSnapshot);
    if (!documentSnapshot.dirty) {
      clearRecoveryTimer(String(documentSnapshot.documentId));
    }
  }
  const openDocumentIds = new Set(snapshot.documents.map((entry) => String(entry.documentId)));
  for (const [id, record] of records) {
    if (!openDocumentIds.has(id)) {
      clearRecoveryTimer(id);
      record.model.dispose();
      records.delete(id);
      documentQueues.delete(id);
    }
  }

  const activeRecord = records.get(String(activeSnapshot.documentId));
  const restoreViewState = activeRecord.viewState
    && activeRecord.viewStateText === activeSnapshot.text
    && samePosition(activeRecord.viewStateCursor, activeSnapshot.cursor)
    && sameSelection(activeRecord.viewStateSelection, activeSnapshot.selection);
  activeDocumentId = activeRecord.id;
  suppressedDocuments.add(activeRecord.id);
  if (editor.getModel() !== activeRecord.model) {
    editor.setModel(activeRecord.model);
  }
  if (restoreViewState) {
    editor.restoreViewState(activeRecord.viewState);
    activeRecord.viewState = null;
    activeRecord.viewStateText = null;
    activeRecord.viewStateCursor = null;
    activeRecord.viewStateSelection = null;
  } else {
    editor.setPosition(activeSnapshot.cursor);
    if (activeSnapshot.selection) {
      editor.setSelection(
        new monaco.Selection(
          activeSnapshot.selection.anchor.lineNumber,
          activeSnapshot.selection.anchor.column,
          activeSnapshot.selection.focus.lineNumber,
          activeSnapshot.selection.focus.column,
        ),
      );
    }
  }
  suppressedDocuments.delete(activeRecord.id);

  updateDocumentStatus(activeSnapshot);
  editor.updateOptions({ readOnly: operationInProgress || Boolean(activeRecord.syncError) });
  if (focusEditor) {
    editor.focus();
  }
  notify("onSnapshot", snapshot);
}

function samePosition(left, right) {
  return left?.lineNumber === right?.lineNumber && left?.column === right?.column;
}

function sameSelection(left, right) {
  if (left === null || right === null) {
    return left === right;
  }
  return samePosition(left?.anchor, right?.anchor)
    && samePosition(left?.focus, right?.focus);
}

async function flushAllDocuments() {
  await Promise.all([...documentQueues.values()]);
  const failed = [...records.values()].find((record) => record.syncError);
  if (failed) {
    throw failed.syncError;
  }
  await flushRecoveryWrites();
}

async function runEditorOperation(status, operation) {
  if (operationInProgress) {
    return false;
  }
  operationInProgress = true;
  notify("onOperationInProgress", true);
  editor.updateOptions({ readOnly: true });
  if (status) {
    notify("onStatus", status);
  }
  try {
    const result = await operation();
    return result === undefined ? true : result;
  } catch (error) {
    notify(
      "onStatus",
      bridgeErrorCode(error) === "cancelled"
        ? "Operation cancelled."
        : `Operation failed: ${bridgeErrorMessage(error)}`,
    );
    return false;
  } finally {
    operationInProgress = false;
    notify("onOperationInProgress", false);
    if (!disposed) {
      const activeRecord = records.get(activeDocumentId);
      editor.updateOptions({ readOnly: Boolean(activeRecord?.syncError) });
    }
  }
}

function runAction(action) {
  const actions = {
    new: () => runEditorOperation("Creating document...", async () => {
      await flushAllDocuments();
      await bridgeInvoke("new_document", {
        request: { language: "rust" },
      }).then((snapshot) => renderSnapshot(snapshot, true));
      notify("onStatus", "New document created.");
    }),
    open: () => runEditorOperation("Opening file...", async () => {
      await flushAllDocuments();
      await bridgeInvoke("open_document", { request: {} })
        .then((snapshot) => renderSnapshot(snapshot, true));
      notify("onStatus", "File opened.");
    }),
    openWorkspace: () => runEditorOperation("Opening project folder...", async () => {
      await flushAllDocuments();
      if (currentSnapshot?.documents.some((document) => document.dirty)) {
        notify("onWorkspaceActionNeedsSave", "open");
        notify("onStatus", "Save all open documents before opening a workspace.");
        return false;
      }
      await bridgeInvoke("open_workspace", { request: {} })
        .then((snapshot) => renderSnapshot(snapshot, true));
      notify("onWorkspaceActionNeedsSave", null);
      notify("onStatus", "Project folder opened. Previous document tabs were closed.");
    }),
    closeWorkspace: () => runEditorOperation("Closing project folder...", async () => {
      await flushAllDocuments();
      if (currentSnapshot?.documents.some((document) => document.dirty)) {
        notify("onWorkspaceActionNeedsSave", "close");
        notify("onStatus", "Save all open documents before closing this workspace.");
        return false;
      }
      await bridgeInvoke("close_workspace").then((snapshot) => renderSnapshot(snapshot, true));
      notify("onWorkspaceActionNeedsSave", null);
      notify("onStatus", "Workspace closed.");
    }),
    reopenClosed: () => runEditorOperation("Reopening recently closed document...", async () => {
      await flushAllDocuments();
      const snapshot = await bridgeInvoke("reopen_closed_document");
      renderSnapshot(snapshot, true);
      const activeDocument = snapshot.documents.find(
        (document) => document.viewId === snapshot.activeViewId,
      );
      notify(
        "onStatus",
        activeDocument ? `${activeDocument.title} reopened.` : "No recently closed document is available.",
      );
    }),
    clearRecentDocuments: () => runEditorOperation("Clearing recent document history...", async () => {
      await flushAllDocuments();
      const snapshot = await bridgeInvoke("clear_recent_documents");
      renderSnapshot(snapshot);
      notify("onStatus", "Recent document history cleared.");
    }),
    save: () => runEditorOperation("Saving document...", () => saveActiveDocument(false)),
    saveAs: () => runEditorOperation("Saving document as...", () => saveActiveDocument(true)),
    exit: () => {
      bridgeInvoke("exit_application").catch((error) => {
        notify("onStatus", `Unable to exit the application: ${bridgeErrorMessage(error)}`);
      });
    },
  };
  actions[action]();
}

async function saveActiveDocument(saveAs) {
  await flushAllDocuments();
  const record = records.get(activeDocumentId);
  if (!record) {
    throw new Error("The active document is no longer open.");
  }
  const snapshot = await saveDocumentRecord(record, saveAs);
  renderSnapshot(snapshot);
  const savedDocument = snapshot.documents.find(
    (document) => document.documentId === Number(record.id),
  );
  if (!savedDocument || savedDocument.dirty) {
    notify("onStatus", "The document changed while it was being saved. Review it before continuing.");
    return false;
  }
  notify("onStatus", saveAs ? "Document saved as." : "Document saved.");
  return true;
}

async function saveDocumentRecord(record, saveAs = false) {
  return bridgeInvoke("save_document", {
    request: {
      documentId: Number(record.id),
      expectedRevision: record.revision,
      saveAs,
    },
  });
}

function saveDocument(documentId) {
  return runEditorOperation("Saving document...", async () => {
    await flushAllDocuments();
    const record = records.get(String(documentId));
    if (!record) {
      throw new Error("The document is no longer open.");
    }
    const snapshot = await saveDocumentRecord(record);
    renderSnapshot(snapshot);
    const savedDocument = snapshot.documents.find(
      (document) => document.documentId === Number(documentId),
    );
    if (!savedDocument || savedDocument.dirty) {
      notify("onStatus", "The document changed while it was being saved. Review it before closing.");
      return false;
    }
    notify("onStatus", "Document saved.");
    return true;
  });
}

function saveAllDocuments() {
  return runEditorOperation("Saving open documents...", async () => {
    await flushAllDocuments();
    if (!currentSnapshot) {
      throw new Error("The editor session is not ready.");
    }
    const dirtyDocumentIds = currentSnapshot.documents
      .filter((document) => document.dirty)
      .map((document) => document.documentId);
    for (const documentId of dirtyDocumentIds) {
      const record = records.get(String(documentId));
      if (!record) {
        throw new Error("An unsaved document is no longer open.");
      }
      const snapshot = await saveDocumentRecord(record);
      renderSnapshot(snapshot);
      const savedDocument = snapshot.documents.find(
        (document) => document.documentId === documentId,
      );
      if (!savedDocument || savedDocument.dirty) {
        notify("onStatus", "A document changed while it was being saved. Review it before continuing.");
        return false;
      }
    }
    notify("onStatus", "All open documents saved.");
    return true;
  });
}

function getSettings() {
  const requestVersion = ++settingsRequestVersion;
  return bridgeInvoke("get_settings").then((catalog) => {
    if (!disposed && requestVersion === settingsRequestVersion) {
      applyEditorSettings(catalog.settings, editor, monaco);
      settingsLoadWarning = catalog.warnings.length > 0
        ? `Settings warning: ${catalog.warnings.join(" ")}`
        : null;
      notify("onStatus", "Settings loaded.");
    }
    return catalog;
  });
}

function saveSettings(settings) {
  const requestVersion = ++settingsRequestVersion;
  return bridgeInvoke("save_settings", { request: { settings } }).then((saved) => {
    if (!disposed && requestVersion === settingsRequestVersion) {
      applyEditorSettings(saved, editor, monaco);
      settingsLoadWarning = null;
      notify("onStatus", "Settings saved.");
    }
    return saved;
  });
}

function loadUserTheme(name) {
  return bridgeInvoke("load_user_theme", { request: { name } });
}

function saveUserTheme(name, settings) {
  return bridgeInvoke("save_user_theme", { request: { name, settings } });
}

function closeDocument(viewId, discardChanges) {
  return runEditorOperation("Closing document...", async () => {
    await flushAllDocuments();
    const record = [...records.values()].find(
      (candidate) => candidate.snapshot.viewId === viewId,
    );
    if (!record) {
      throw new Error("The document is no longer open.");
    }
    const snapshot = await bridgeInvoke("close_document", {
      request: {
        viewId,
        expectedRevision: record.revision,
        discardChanges,
      },
    });
    renderSnapshot(snapshot, true);
    notify("onStatus", `${record.snapshot.title} closed.`);
    return true;
  });
}

function searchWorkspace(request) {
  return runEditorOperation("Searching workspace files...", async () => {
    await flushAllDocuments();
    const response = await bridgeInvoke("search_workspace", {
      request,
    });
    const status = response.cancelled
      ? "Search cancelled."
      : response.skippedFiles > 0
        ? `Found ${response.hits.length} matches; ${response.skippedFiles} non-text or unreadable files could not be searched.`
        : `Found ${response.hits.length} matches.`;
    notify("onStatus", status);
    return response;
  }).then((response) => response && typeof response === "object" ? response : null);
}

function cancelSearch(searchId) {
  return bridgeInvoke("cancel_search", {
    request: { searchId },
  }).then((cancelled) => Boolean(cancelled));
}

function setSearchHighlights(hits, activeIndex) {
  const hitsByDocument = new Map();
  for (const [index, hit] of (hits ?? []).entries()) {
    if (hit.documentId === null) continue;
    const documentHits = hitsByDocument.get(String(hit.documentId)) ?? [];
    documentHits.push({ hit, index });
    hitsByDocument.set(String(hit.documentId), documentHits);
  }

  for (const [documentId, record] of records) {
    const decorations = (hitsByDocument.get(documentId) ?? []).map(({ hit, index }) => ({
      range: new monaco.Range(
        hit.lineNumber,
        hit.column,
        hit.lineNumber,
        hit.column + hit.matchLength,
      ),
      options: {
        className: index === activeIndex
          ? "functor-search-match-active"
          : "functor-search-match-inactive",
        stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
      },
    }));
    record.searchDecorations = record.model.deltaDecorations(
      record.searchDecorations,
      decorations,
    );
  }
}

function replaceSearchResults(hits, replacement) {
  return runEditorOperation("Replacing search matches...", async () => {
    if (typeof replacement !== "string") {
      throw new Error("Replacement text must be a string.");
    }
    await flushAllDocuments();
    const openHits = hits.filter((hit) => hit.documentId !== null);
    const unopenedHits = hits.filter((hit) => hit.documentId === null);
    const plan = planOpenSearchReplacements(
      openHits,
      (documentId) => records.get(documentId)?.revision,
    );
    const prepared = [];
    for (const group of plan.groups) {
      const record = records.get(group.documentId);
      if (!record) {
        throw new Error("Search results are stale. Search again before replacing.");
      }
      const edits = group.hits
        .map((hit) => {
          const range = new monaco.Range(
            hit.lineNumber,
            hit.column,
            hit.lineNumber,
            hit.column + hit.matchLength,
          );
          if (record.model.getValueInRange(range) !== hit.matchedText) {
            throw new Error("A search match changed. Search again before replacing.");
          }
          return { hit, range };
        });
      prepared.push({ record, edits });
    }

    const replacementBytes = new TextEncoder().encode(replacement).length;
    for (const { edits } of prepared) {
      for (let start = 0; start < edits.length; start += 200) {
        if (replacementBytes * Math.min(200, edits.length - start) > 1_048_576) {
          throw new Error("Replacement text exceeds the 1 MiB edit limit for one batch.");
        }
      }
    }

    let closedResponse = null;
    if (unopenedHits.length > 0) {
      const workspaceRoot = unopenedHits[0].workspaceRoot;
      if (!workspaceRoot) {
        throw new Error("A workspace search result is missing its workspace identity.");
      }
      const filesByPath = new Map();
      for (const hit of unopenedHits) {
        if (
          hit.workspaceRoot !== workspaceRoot ||
          !hit.sourceFingerprint ||
          hit.revision !== null
        ) {
          throw new Error("Search results are stale. Search again before replacing.");
        }
        let file = filesByPath.get(hit.relativePath);
        if (!file) {
          file = {
            relativePath: hit.relativePath,
            sourceFingerprint: hit.sourceFingerprint,
            matches: [],
          };
          filesByPath.set(hit.relativePath, file);
        } else if (file.sourceFingerprint !== hit.sourceFingerprint) {
          throw new Error("Search results for a file span multiple source versions.");
        }
        file.matches.push({
          rangeOffset: hit.rangeOffset,
          rangeLength: hit.rangeLength,
          matchedText: hit.matchedText,
        });
      }
      closedResponse = await bridgeInvoke("replace_workspace_search_results", {
        request: {
          workspaceRoot,
          replacement,
          files: [...filesByPath.values()],
        },
      });
      if (
        !closedResponse ||
        closedResponse.replaced !== unopenedHits.length ||
        !Number.isSafeInteger(closedResponse.filesWritten) ||
        closedResponse.filesWritten < 0
      ) {
        throw new Error("The host returned an invalid workspace replacement result.");
      }
    }

    let replaced = 0;
    for (const { record, edits } of prepared) {
      record.model.pushStackElement();
      for (let start = 0; start < edits.length; start += 200) {
        const batch = edits.slice(start, start + 200);
        record.model.pushEditOperations(
          null,
          batch.map(({ range }) => ({ range, text: replacement })),
          () => null,
        );
        const queue = documentQueues.get(record.id);
        if (queue) await queue;
        if (record.syncError) throw record.syncError;
      }
      record.model.pushStackElement();
      replaced += edits.length;
    }
    replaced += unopenedHits.length;
    setSearchHighlights(null, -1);
    const status = [`Replaced ${replaced} matches.`];
    if (closedResponse?.filesWritten > 0) {
      status.push(
        `Updated ${closedResponse.filesWritten} unopened workspace file${closedResponse.filesWritten === 1 ? "" : "s"} on disk.`,
      );
    }
    if (prepared.length > 0) {
      status.push("Open documents have unsaved changes; save them to write those files.");
    }
    notify("onStatus", status.join(" "));
    return { replaced };
  }).then((result) =>
    result && typeof result === "object" ? result : null);
}

function openWorkspaceSearchResult(hit) {
  return runEditorOperation(`Opening ${hit.relativePath}...`, async () => {
    await flushAllDocuments();
    await openWorkspaceSearchResultSnapshot(hit);
  });
}

async function openWorkspaceSearchResultSnapshot(hit) {
  const snapshot = await bridgeInvoke("open_workspace_search_result", {
    request: toOpenWorkspaceSearchResultRequest(hit),
  });
  renderSnapshot(snapshot, true);
  notify(
    "onStatus",
    `Opened ${hit.relativePath} at line ${hit.lineNumber}.`,
  );
  return snapshot;
}

function switchDocument(viewId) {
  runEditorOperation("Switching document...", async () => {
    await flushAllDocuments();
    const snapshot = await bridgeInvoke("switch_document", {
      request: { viewId },
    });
    renderSnapshot(snapshot, true);
    notify("onStatus", "Document switched.");
  });
}

function setDocumentLanguage(documentId, language) {
  return runEditorOperation("Changing language mode...", async () => {
    await flushAllDocuments();
    const record = records.get(String(documentId));
    if (!record) {
      throw new Error("The selected document is no longer open.");
    }
    const snapshot = await bridgeInvoke("set_document_language", {
      request: {
        documentId,
        expectedRevision: record.revision,
        language,
      },
    });
    renderSnapshot(snapshot);
    notify("onStatus", "Language mode changed.");
  });
}

function loadWorkspaceDirectory(relativePath) {
  return runEditorOperation(`Loading ${relativePath}...`, async () => {
    await flushAllDocuments();
    const snapshot = await bridgeInvoke("load_workspace_directory", {
      request: { relativePath },
    });
    renderSnapshot(snapshot);
    notify("onStatus", `Loaded ${relativePath}.`);
  });
}

function openWorkspaceFile(relativePath) {
  return runEditorOperation(`Opening ${relativePath}...`, async () => {
    await flushAllDocuments();
    const snapshot = await bridgeInvoke("open_workspace_file", {
      request: { relativePath },
    });
    renderSnapshot(snapshot, true);
    notify("onStatus", `Opened ${relativePath}.`);
  });
}

export function createEditorBridge(editorElement, bridgeCallbacks) {
  disposed = false;
  callbacks = bridgeCallbacks;
  notify("onLanguageOptions", languageOptions);
  records = new Map();
  documentQueues = new Map();
  recoveryTimers = new Map();
  recoveryWrites = new Map();
  suppressedDocuments = new Set();
  modelDocumentIds = new WeakMap();
  activeDocumentId = null;
  currentSnapshot = null;
  operationInProgress = false;
  workspacePersistenceWarning = null;
  settingsLoadWarning = null;
  settingsRequestVersion = 0;
  editor = monaco.editor.create(editorElement, {
    theme: "vs-dark",
    automaticLayout: true,
    minimap: { enabled: false },
    fontSize: 14,
    tabSize: 4,
    insertSpaces: true,
    scrollBeyondLastLine: false,
    contextmenu: false,
    ariaLabel: "Functor code editor",
  });
  attachCursorSelectionHandler();
  attachGraphemeInputHandler();

  getSettings().catch((error) => {
    settingsLoadWarning = `Settings could not be loaded: ${bridgeErrorMessage(error)}`;
    notify("onStatus", "The editor is ready.");
  });

  bridgeInvoke("get_snapshot")
    .then(async (snapshot) => {
      if (disposed) {
        return;
      }
      operationInProgress = true;
      notify("onOperationInProgress", true);
      try {
        renderSnapshot(snapshot);
        const { workspace: restored, warnings: startupWarnings } =
          await restoreStartupDocuments(bridgeInvoke, renderSnapshot, bridgeErrorMessage);
        const status = restored?.snapshot.workspaceError
          ? `The previous workspace could not be restored: ${restored.snapshot.workspaceError}`
          : restored?.snapshot.workspace
            ? "Workspace and open tabs restored."
            : "Ready.";
        notify(
          "onStatus",
          startupWarnings.length > 0
            ? `${status} ${startupWarnings.join(" ")}`
            : status,
        );
      } catch (error) {
        notify(
          "onStatus",
          `Workspace and unsaved work could not be restored: ${bridgeErrorMessage(error)}`,
        );
      } finally {
        operationInProgress = false;
        notify("onOperationInProgress", false);
        if (!disposed) {
          const activeRecord = records.get(activeDocumentId);
          editor.updateOptions({ readOnly: Boolean(activeRecord?.syncError) });
        }
      }
    })
    .catch((error) => {
      notify("onStatus", `Unable to load the editor session: ${bridgeErrorMessage(error)}`);
    });

  return {
    runAction,
    switchDocument,
    setDocumentLanguage,
    saveDocument,
    saveAllDocuments,
    closeDocument,
    searchWorkspace,
    cancelSearch,
    openWorkspaceSearchResult,
    setSearchHighlights,
    replaceSearchResults,
    loadWorkspaceDirectory,
    openWorkspaceFile,
    getSettings,
    saveSettings,
    loadUserTheme,
    saveUserTheme,
    dispose() {
      for (const documentId of [...recoveryTimers.keys()]) {
        clearRecoveryTimer(documentId);
      }
      disposed = true;
      for (const record of records.values()) {
        record.model.dispose();
      }
      records.clear();
      editor.dispose();
    },
  };
}

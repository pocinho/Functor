import assert from "node:assert/strict";
import { test } from "node:test";
import { toOpenWorkspaceSearchResultRequest } from "../src/search_result_request.js";

test("search result navigation sends only fields accepted by the host command", () => {
  const hit = {
    relativePath: "src/main.rs",
    lineNumber: 12,
    column: 5,
    matchLength: 7,
    documentId: 9,
    revision: 4,
    sourceFingerprint: null,
    workspaceRoot: "C:\\project",
    preview: "let functor = true;",
  };

  assert.deepEqual(toOpenWorkspaceSearchResultRequest(hit), {
    relativePath: "src/main.rs",
    lineNumber: 12,
    column: 5,
    matchLength: 7,
    documentId: 9,
    expectedRevision: 4,
    sourceFingerprint: null,
    workspaceRoot: "C:\\project",
  });
});

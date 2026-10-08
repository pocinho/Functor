/**
 * @param {import("./protocol.js").WorkspaceSearchHit} hit
 * @returns {import("./protocol.js").OpenWorkspaceSearchResultRequest}
 */
export function toOpenWorkspaceSearchResultRequest(hit) {
  return {
    relativePath: hit.relativePath,
    lineNumber: hit.lineNumber,
    column: hit.column,
    matchLength: hit.matchLength,
    documentId: hit.documentId,
    expectedRevision: hit.revision,
    sourceFingerprint: hit.sourceFingerprint,
    workspaceRoot: hit.workspaceRoot,
  };
}

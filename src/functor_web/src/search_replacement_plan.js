export function planOpenSearchReplacements(hits, getRevision) {
  const groupedHits = new Map();
  let skipped = 0;
  for (const hit of hits) {
    if (hit.documentId === null) {
      skipped += 1;
      continue;
    }
    const documentId = String(hit.documentId);
    const documentHits = groupedHits.get(documentId) ?? [];
    documentHits.push(hit);
    groupedHits.set(documentId, documentHits);
  }

  const groups = [];
  for (const [documentId, documentHits] of groupedHits) {
    const revision = documentHits[0].revision;
    if (revision === null || getRevision(documentId) !== revision) {
      throw new Error("Search results are stale. Search again before replacing.");
    }
    if (documentHits.some((hit) => hit.revision !== revision)) {
      throw new Error("Search results span multiple document revisions.");
    }
    groups.push({
      documentId,
      revision,
      hits: [...documentHits].sort((left, right) => right.rangeOffset - left.rangeOffset),
    });
  }

  return { groups, skipped };
}

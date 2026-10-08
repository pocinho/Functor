import assert from "node:assert/strict";
import { test } from "node:test";
import { planOpenSearchReplacements } from "../src/search_replacement_plan.js";

function hit(documentId, rangeOffset, revision = 3) {
  return { documentId, rangeOffset, revision };
}

test("replacement plans skip unopened results and order edits from end to start", () => {
  const plan = planOpenSearchReplacements(
    [hit(7, 0), hit(null, 4, null), hit(7, 12)],
    () => 3,
  );

  assert.equal(plan.skipped, 1);
  assert.deepEqual(plan.groups, [{
    documentId: "7",
    revision: 3,
    hits: [hit(7, 12), hit(7, 0)],
  }]);
});

test("replacement planning rejects a document revision changed since search", () => {
  assert.throws(
    () => planOpenSearchReplacements([hit(7, 0)], () => 4),
    /Search results are stale/,
  );
});

test("replacement planning rejects hits captured at different revisions", () => {
  assert.throws(
    () => planOpenSearchReplacements([hit(7, 0, 3), hit(7, 12, 4)], () => 3),
    /multiple document revisions/,
  );
});

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  graphemeBoundaries,
  nextGraphemeBoundary,
  previousGraphemeBoundary,
} from "../src/grapheme_boundaries.js";

test("finds UTF-16 boundaries for emoji, combining marks, and joined emoji", () => {
  const segments = ["A", "🦀", "e\u0301", "👨‍👩‍👧‍👦", "Z"];
  const text = segments.join("");
  const expected = [0];
  for (const segment of segments) {
    expected.push(expected.at(-1) + segment.length);
  }

  assert.deepEqual(graphemeBoundaries(text), expected);
});

test("moves to complete grapheme boundaries from inside a combining sequence", () => {
  const boundaries = graphemeBoundaries("A🦀e\u0301Z");

  assert.equal(previousGraphemeBoundary(boundaries, 5), 3);
  assert.equal(previousGraphemeBoundary(boundaries, 4), 3);
  assert.equal(previousGraphemeBoundary(boundaries, 1), 0);
  assert.equal(previousGraphemeBoundary(boundaries, 0), null);
  assert.equal(nextGraphemeBoundary(boundaries, 3), 5);
  assert.equal(nextGraphemeBoundary(boundaries, 4), 5);
  assert.equal(nextGraphemeBoundary(boundaries, 5), 6);
  assert.equal(nextGraphemeBoundary(boundaries, 6), null);
});

import assert from "node:assert/strict";
import { test } from "node:test";
import { mapMonacoSelection } from "../src/monaco_selection.js";

test("maps stale cursor and selection positions onto the updated Monaco model", () => {
  const model = {
    validatePosition({ lineNumber, column }) {
      const validLine = Math.max(1, Math.min(lineNumber, 1));
      return {
        lineNumber: validLine,
        column: Math.max(1, Math.min(column, 2)),
      };
    },
  };

  assert.deepEqual(
    mapMonacoSelection(
      model,
      { lineNumber: 1, column: 10 },
      {
        selectionStartLineNumber: 1,
        selectionStartColumn: 1,
        positionLineNumber: 1,
        positionColumn: 10,
      },
    ),
    {
      cursor: { lineNumber: 1, column: 2 },
      selection: {
        anchor: { lineNumber: 1, column: 1 },
        focus: { lineNumber: 1, column: 2 },
      },
    },
  );
});

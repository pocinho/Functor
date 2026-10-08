export function mapMonacoSelection(model, position, selection) {
  const validPosition = model?.validatePosition(position ?? { lineNumber: 1, column: 1 })
    ?? position
    ?? { lineNumber: 1, column: 1 };
  const validSelection = selection
    ? {
        anchor: model?.validatePosition({
          lineNumber: selection.selectionStartLineNumber,
          column: selection.selectionStartColumn,
        }) ?? {
          lineNumber: selection.selectionStartLineNumber,
          column: selection.selectionStartColumn,
        },
        focus: model?.validatePosition({
          lineNumber: selection.positionLineNumber,
          column: selection.positionColumn,
        }) ?? {
          lineNumber: selection.positionLineNumber,
          column: selection.positionColumn,
        },
      }
    : null;

  return {
    cursor: {
      lineNumber: validPosition.lineNumber,
      column: validPosition.column,
    },
    selection: validSelection,
  };
}

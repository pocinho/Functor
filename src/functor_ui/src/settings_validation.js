const colorFields = [
  "background",
  "foreground",
  "selection",
  "cursor",
  "lineNumber",
  "gutterBackground",
  "gutterSeparator",
  "editorBorder",
  "diagnosticError",
  "diagnosticWarning",
  "diagnosticInfo",
  "syntaxKeyword",
  "syntaxString",
  "syntaxComment",
  "syntaxNumber",
  "syntaxType",
  "syntaxFunction",
  "resizeHandleColor",
  "commandPaletteShadowColor",
  "workspaceSeparatorColor",
  "measurementColor",
];

const typographyFields = [
  ["workspaceFontSize", "Workspace font size", 6, 48],
  ["editorFontSize", "Editor font size", 6, 72],
  ["editorLineHeight", "Editor line height", 6, 128],
  ["commandPaletteFontSize", "Command palette font size", 6, 48],
  ["welcomeTitleFontSize", "Welcome title font size", 6, 96],
];

const geometryFields = [
  ["shellPaddingX", "Horizontal shell padding", 0, 100],
  ["shellPaddingY", "Vertical shell padding", 0, 100],
  ["workspaceSidebarWidth", "Workspace sidebar width", 160, 800],
  ["editorBorderWidth", "Editor border width", 0, 8],
  ["documentTabMinHeight", "Document tab minimum height", 20, 128],
  ["documentTabPaddingHorizontal", "Document tab horizontal padding", 0, 64],
  ["documentTabPaddingVertical", "Document tab vertical padding", 0, 64],
  ["commandPaletteWidth", "Command palette width", 240, 1600],
  ["commandPalettePadding", "Command palette padding", 0, 64],
  ["commandPaletteMaxHeight", "Command palette maximum height", 120, 1600],
];

export function validateSettings(settings) {
  const errors = [];
  if (!settings || typeof settings !== "object") {
    return ["Settings are missing."];
  }
  if (settings.schemaVersion !== 1) {
    errors.push("Unsupported settings schema version.");
  }
  if (!["graphiteDark", "graphiteLight", "custom"].includes(settings.preset)) {
    errors.push("Select a supported theme preset.");
  }
  for (const field of colorFields) {
    const value = settings.colors?.[field];
    if (typeof value !== "string" || !/^#?(?:[\da-fA-F]{6}|[\da-fA-F]{8})$/.test(value)) {
      errors.push(`${field} must be a 6- or 8-digit hexadecimal color.`);
    }
  }
  for (const [field, label] of [
    ["editorFontFamily", "Editor font family"],
    ["editorFallbackFontFamily", "Editor fallback font family"],
    ["uiFontFamily", "UI font family"],
  ]) {
    const value = settings.typography?.[field];
    if (typeof value !== "string" || !value.trim() || value.length > 256 || /[\u0000-\u001f\u007f]/.test(value)) {
      errors.push(`${label} must be a non-empty font family of at most 256 characters.`);
    }
  }
  for (const [field, label, minimum, maximum] of typographyFields) {
    const value = settings.typography?.[field];
    if (typeof value !== "number" || !Number.isFinite(value) || value < minimum || value > maximum) {
      errors.push(`${label} must be between ${minimum} and ${maximum}.`);
    }
  }
  const tabSize = settings.typography?.editorTabSize;
  if (!Number.isInteger(tabSize) || tabSize < 1 || tabSize > 16) {
    errors.push("Editor tab size must be between 1 and 16.");
  }
  for (const [field, label, minimum, maximum] of geometryFields) {
    const value = settings.geometry?.[field];
    if (typeof value !== "number" || !Number.isFinite(value) || value < minimum || value > maximum) {
      errors.push(`${label} must be between ${minimum} and ${maximum}.`);
    }
  }
  return errors;
}

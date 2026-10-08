import assert from "node:assert/strict";
import { test } from "node:test";

import {
  argbToCssColor,
  monacoThemeDefinition,
  settingsCssVariables,
} from "../src/settings_theme.js";
import {
  settingsColorFromPicker,
  settingsColorPickerValue,
  settingsFontFamilies,
  settingsWithPreset,
} from "../../functor_ui/src/settings_draft.js";
import { validateSettings } from "../../functor_ui/src/settings_validation.js";

function settings(preset = "graphiteDark") {
  return {
    schemaVersion: 1,
    preset,
    colors: {
      background: "#FF17151B",
      foreground: "#FFE7E3ED",
      selection: "#705E4A8F",
      cursor: "#FFD8C9F0",
      lineNumber: "#FF8F8799",
      gutterBackground: "#FF1E1B24",
      gutterSeparator: "#FF393241",
      editorBorder: "#FF393241",
      diagnosticError: "#FFFF7185",
      diagnosticWarning: "#FFE4B45D",
      diagnosticInfo: "#FF72C5E8",
      syntaxKeyword: "#FFC7A6F7",
      syntaxString: "#FFE5A58F",
      syntaxComment: "#FF7CA889",
      syntaxNumber: "#FF9DD6C8",
      syntaxType: "#FF7CC4C7",
      syntaxFunction: "#FFE1B1E8",
      resizeHandleColor: "#FF333333",
      commandPaletteShadowColor: "#66000000",
      workspaceSeparatorColor: "#6EA0A0A0",
      measurementColor: "#FFFFFFFF",
    },
    typography: {
      editorFontFamily: "Consolas",
      editorFallbackFontFamily: "Segoe UI Emoji",
      uiFontFamily: "Segoe UI",
      workspaceFontSize: 12,
      editorFontSize: 13.333,
      editorLineHeight: 16,
      editorTabSize: 4,
      commandPaletteFontSize: 14,
      welcomeTitleFontSize: 28,
    },
    geometry: {
      shellPaddingX: 24,
      shellPaddingY: 20,
      workspaceSidebarWidth: 256,
      editorBorderWidth: 1,
      documentTabMinHeight: 41,
      documentTabPaddingHorizontal: 14.4,
      documentTabPaddingVertical: 10.4,
      commandPaletteWidth: 672,
      commandPalettePadding: 10.4,
      commandPaletteMaxHeight: 544,
    },
  };
}

test("validates settings drafts and rejects invalid theme values", () => {
  const draft = settings();
  assert.deepEqual(validateSettings(draft), []);

  draft.colors.cursor = "not-a-color";
  draft.typography.editorTabSize = 0;
  draft.geometry.workspaceSidebarWidth = Number.POSITIVE_INFINITY;
  assert.equal(validateSettings(draft).length, 3);
});

test("selecting a preset replaces its draft and Custom preserves edited values", () => {
  const current = settings();
  current.typography.editorFontFamily = "Cascadia Code";
  const light = settings("graphiteLight");

  assert.equal(settingsWithPreset(current, [light], "graphiteLight"), light);
  assert.deepEqual(settingsWithPreset(current, [light], "custom"), {
    ...current,
    preset: "custom",
  });
  assert.equal(settingsWithPreset(current, [], "graphiteLight"), null);
});

test("color picker values preserve ARGB alpha when updating RGB", () => {
  assert.equal(settingsColorPickerValue("#804A8F5E"), "#4A8F5E");
  assert.equal(settingsColorPickerValue("4A8F5E"), "#4A8F5E");
  assert.equal(settingsColorPickerValue("#invalid"), null);
  assert.equal(settingsColorFromPicker("#804A8F5E", "#123456"), "#80123456");
  assert.equal(settingsColorFromPicker("4A8F5E", "#123456"), "123456");
  assert.equal(settingsColorFromPicker("#invalid", "#123456"), null);
});

test("font family picker offers common UI and editor fonts", () => {
  assert.ok(settingsFontFamilies.includes("Segoe UI"));
  assert.ok(settingsFontFamilies.includes("Consolas"));
  assert.ok(settingsFontFamilies.includes("Cascadia Code"));
  assert.ok(settingsFontFamilies.includes("monospace"));
});

test("converts persisted ARGB colors to CSS RGBA colors", () => {
  assert.equal(argbToCssColor("#705E4A8F"), "#5E4A8F70");
  assert.equal(argbToCssColor("FF17151B"), "#17151BFF");
  assert.throws(() => argbToCssColor("#xyz"), /Invalid theme color/);
});

test("projects theme typography, shell geometry, and syntax colors", () => {
  const draft = settings("graphiteLight");
  draft.colors.background = "#FFE7E5EA";
  draft.colors.selection = "#604E3A78";
  draft.colors.syntaxKeyword = "#FF633F9A";
  const variables = settingsCssVariables(draft);
  const theme = monacoThemeDefinition(draft);

  assert.equal(variables.get("--theme-color-scheme"), "light");
  assert.equal(variables.get("--theme-workspace-sidebar-width"), "256px");
  assert.equal(theme.base, "vs");
  assert.equal(theme.colors["editor.background"], "#E7E5EAFF");
  assert.equal(theme.colors["editor.selectionBackground"], "#4E3A7860");
  assert.equal(theme.rules[0].foreground, "633F9A");
});

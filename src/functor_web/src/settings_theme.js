const settingsColors = [
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

export function argbToCssColor(value) {
  if (typeof value !== "string") {
    throw new TypeError("A theme color must be a string.");
  }
  const hex = value.startsWith("#") ? value.slice(1) : value;
  if (!/^(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(hex)) {
    throw new TypeError(`Invalid theme color: ${value}`);
  }
  return hex.length === 8
    ? `#${hex.slice(2)}${hex.slice(0, 2)}`.toUpperCase()
    : `#${hex}`.toUpperCase();
}

function rgbColor(value) {
  return argbToCssColor(value).slice(1, 7);
}

function isLightColor(value) {
  const color = rgbColor(value);
  const channels = [0, 2, 4].map((offset) =>
    Number.parseInt(color.slice(offset, offset + 2), 16));
  const [red, green, blue] = channels.map((channel) => {
    const normalized = channel / 255;
    return normalized <= 0.04045
      ? normalized / 12.92
      : ((normalized + 0.055) / 1.055) ** 2.4;
  });
  return (0.2126 * red + 0.7152 * green + 0.0722 * blue) > 0.45;
}

export function settingsCssVariables(settings) {
  const colors = settings.colors;
  const typography = settings.typography;
  const geometry = settings.geometry;
  const variables = new Map(
    settingsColors.map((name) => [`--theme-${name.replace(/[A-Z]/g, (char) => `-${char.toLowerCase()}`)}`, argbToCssColor(colors[name])]),
  );
  variables.set("--theme-font-ui", typography.uiFontFamily);
  variables.set("--theme-font-size-workspace", `${typography.workspaceFontSize}px`);
  variables.set("--theme-font-size-command-palette", `${typography.commandPaletteFontSize}px`);
  variables.set("--theme-font-size-welcome", `${typography.welcomeTitleFontSize}px`);
  variables.set("--theme-shell-padding-x", `${geometry.shellPaddingX}px`);
  variables.set("--theme-shell-padding-y", `${geometry.shellPaddingY}px`);
  variables.set("--theme-workspace-sidebar-width", `${geometry.workspaceSidebarWidth}px`);
  variables.set("--theme-editor-border-width", `${geometry.editorBorderWidth}px`);
  variables.set("--theme-tab-min-height", `${geometry.documentTabMinHeight}px`);
  variables.set("--theme-tab-padding-x", `${geometry.documentTabPaddingHorizontal}px`);
  variables.set("--theme-tab-padding-y", `${geometry.documentTabPaddingVertical}px`);
  variables.set("--theme-command-palette-width", `${geometry.commandPaletteWidth}px`);
  variables.set("--theme-command-palette-padding", `${geometry.commandPalettePadding}px`);
  variables.set("--theme-command-palette-max-height", `${geometry.commandPaletteMaxHeight}px`);
  variables.set(
    "--theme-color-scheme",
    settings.preset === "graphiteLight"
      || (settings.preset === "custom" && isLightColor(colors.background))
      ? "light"
      : "dark",
  );
  return variables;
}

export function monacoThemeDefinition(settings) {
  const colors = settings.colors;
  const light = settings.preset === "graphiteLight"
    || (settings.preset === "custom" && isLightColor(colors.background));
  return {
    base: light ? "vs" : "vs-dark",
    inherit: true,
    rules: [
      { token: "keyword", foreground: rgbColor(colors.syntaxKeyword) },
      { token: "string", foreground: rgbColor(colors.syntaxString) },
      { token: "comment", foreground: rgbColor(colors.syntaxComment) },
      { token: "number", foreground: rgbColor(colors.syntaxNumber) },
      { token: "type", foreground: rgbColor(colors.syntaxType) },
      { token: "function", foreground: rgbColor(colors.syntaxFunction) },
    ],
    colors: {
      "editor.background": argbToCssColor(colors.background),
      "editor.foreground": argbToCssColor(colors.foreground),
      "editor.selectionBackground": argbToCssColor(colors.selection),
      "editorCursor.foreground": argbToCssColor(colors.cursor),
      "editorLineNumber.foreground": argbToCssColor(colors.lineNumber),
      "editorGutter.background": argbToCssColor(colors.gutterBackground),
      "editorOverviewRuler.border": argbToCssColor(colors.gutterSeparator),
      "editorWidget.border": argbToCssColor(colors.editorBorder),
      "editorError.foreground": argbToCssColor(colors.diagnosticError),
      "editorWarning.foreground": argbToCssColor(colors.diagnosticWarning),
      "editorInfo.foreground": argbToCssColor(colors.diagnosticInfo),
    },
  };
}

export function applyEditorSettings(settings, editor, monaco) {
  for (const [name, value] of settingsCssVariables(settings)) {
    document.documentElement.style.setProperty(name, value);
  }
  document.documentElement.dataset.theme = settings.preset;
  monaco.editor.defineTheme("functor-settings", monacoThemeDefinition(settings));
  editor.updateOptions({
    theme: "functor-settings",
    fontFamily: [
      settings.typography.editorFontFamily,
      settings.typography.editorFallbackFontFamily,
      "monospace",
    ].join(", "),
    fontSize: settings.typography.editorFontSize,
    lineHeight: Math.round(settings.typography.editorLineHeight),
    tabSize: settings.typography.editorTabSize,
  });
}

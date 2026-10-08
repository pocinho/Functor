export const settingsFontFamilies = [
  "Arial",
  "Aptos",
  "Bahnschrift",
  "Calibri",
  "Cambria",
  "Candara",
  "Cascadia Code",
  "Cascadia Mono",
  "Comic Sans MS",
  "Consolas",
  "Constantia",
  "Corbel",
  "Courier New",
  "Franklin Gothic Medium",
  "Gabriola",
  "Georgia",
  "Impact",
  "Leelawadee UI",
  "Lucida Console",
  "Microsoft Sans Serif",
  "Palatino Linotype",
  "Segoe UI",
  "Segoe UI Emoji",
  "Segoe UI Symbol",
  "Sitka",
  "Tahoma",
  "Times New Roman",
  "Trebuchet MS",
  "Verdana",
  "monospace",
  "sans-serif",
  "serif",
  "system-ui",
];

export function settingsWithPreset(current, presets, preset) {
  if (preset === "custom") {
    return { ...current, preset };
  }
  return presets.find((candidate) => candidate.preset === preset) ?? null;
}

const hexColorPattern = /^#?(?:[\da-fA-F]{6}|[\da-fA-F]{8})$/;

export function settingsColorPickerValue(value) {
  if (typeof value !== "string" || !hexColorPattern.test(value)) {
    return null;
  }
  const hex = value.startsWith("#") ? value.slice(1) : value;
  return `#${hex.slice(hex.length === 8 ? 2 : 0).toUpperCase()}`;
}

export function settingsColorFromPicker(current, picked) {
  if (
    typeof current !== "string"
    || !hexColorPattern.test(current)
    || typeof picked !== "string"
    || !/^#[\da-fA-F]{6}$/.test(picked)
  ) {
    return null;
  }
  const prefix = current.startsWith("#") ? "#" : "";
  const hex = prefix ? current.slice(1) : current;
  const alpha = hex.length === 8 ? hex.slice(0, 2) : "";
  return `${prefix}${alpha}${picked.slice(1).toUpperCase()}`;
}

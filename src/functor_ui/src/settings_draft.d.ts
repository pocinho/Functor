export const settingsFontFamilies: readonly string[];

export function settingsWithPreset<T extends { preset: string }>(
  current: T,
  presets: T[],
  preset: T["preset"],
): T | null;

export function settingsColorPickerValue(value: string): string | null;

export function settingsColorFromPicker(current: string, picked: string): string | null;

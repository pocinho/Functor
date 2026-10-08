export function getMonacoLanguageOptions(languages) {
  const options = [{ id: "plainText", label: "Plain Text" }];
  const ids = new Set(["plaintext", "plainText"]);

  for (const language of languages) {
    if (!language.id || ids.has(language.id)) {
      continue;
    }
    ids.add(language.id);
    const alias = language.aliases?.find(Boolean);
    options.push({
      id: language.id,
      label: alias ?? language.id,
    });
  }

  return options.sort((left, right) =>
    left.id === "plainText"
      ? -1
      : right.id === "plainText"
        ? 1
        : left.label.localeCompare(right.label) || left.id.localeCompare(right.id));
}

export function toMonacoLanguage(language, registeredLanguageIds) {
  if (language === "plainText") {
    return "plaintext";
  }
  return registeredLanguageIds.has(language) ? language : "plaintext";
}

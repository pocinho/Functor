export const jsonMonarchDefinition = {
  defaultToken: "invalid",
  tokenPostfix: ".json",
  tokenizer: {
    root: [
      [/\s+/, ""],
      [/"(?:[^"\\]|\\.)*"/, "string"],
      [/"(?:[^"\\]|\\.)*$/, "string.invalid"],
      [/-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/, "number"],
      [/\b(?:true|false|null)\b/, "keyword"],
      [/[{}\[\]]/, "delimiter.bracket"],
      [/[,:]/, "delimiter"],
      [/[^\s{}\[\],:]+/, "invalid"],
    ],
  },
};

export function registerJsonLanguage(monaco) {
  if (!monaco.languages.getLanguages().some((language) => language.id === "json")) {
    monaco.languages.register({
      id: "json",
      extensions: [".json"],
      aliases: ["JSON"],
    });
  }
  return monaco.languages.setMonarchTokensProvider("json", jsonMonarchDefinition);
}

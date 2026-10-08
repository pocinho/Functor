import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";

import { jsonMonarchDefinition, registerJsonLanguage } from "../src/json_language.js";
import {
  getMonacoLanguageOptions,
  toMonacoLanguage,
} from "../src/monaco_language.js";

test("document languages map to their registered Monaco language ids", () => {
  const registeredIds = new Set([
    "rust",
    "csharp",
    "fsharp",
    "json",
    "markdown",
    "typescript",
    "coffeescript",
  ]);
  for (const [language, expected] of [
    ["rust", "rust"],
    ["csharp", "csharp"],
    ["fsharp", "fsharp"],
    ["json", "json"],
    ["markdown", "markdown"],
    ["plainText", "plaintext"],
  ]) {
    assert.equal(toMonacoLanguage(language, registeredIds), expected);
  }
  assert.equal(toMonacoLanguage("unknown", registeredIds), "plaintext");
});

test("language mode options expose registered ids with readable aliases", () => {
  assert.deepEqual(
    getMonacoLanguageOptions([
      { id: "rust", aliases: ["Rust", "rust"] },
      { id: "csharp", aliases: ["C#", "csharp"] },
      { id: "rust", aliases: ["Rust"] },
    ]),
    [
      { id: "plainText", label: "Plain Text" },
      { id: "csharp", label: "C#" },
      { id: "rust", label: "Rust" },
    ],
  );
});

test("Rust extension detection matches Monaco's complete registered language catalogue", () => {
  const definitionsRoot = new URL(
    "../node_modules/monaco-editor/esm/vs/languages/definitions/",
    import.meta.url,
  );
  const monacoDefinitions = new Map();

  for (const entry of readdirSync(definitionsRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const registrationPath = new URL(`${entry.name}/register.js`, definitionsRoot);
    let source;
    try {
      source = readFileSync(registrationPath, "utf8");
    } catch (error) {
      if (error?.code === "ENOENT") continue;
      throw error;
    }
    for (const [, registration] of source.matchAll(
      /registerLanguage\s*\(\s*\{([\s\S]*?)\}\s*\);/g,
    )) {
      const id = registration.match(/\bid:\s*["']([^"']+)["']/)?.[1];
      if (!id) continue;
      const extensions = registration.match(/\bextensions:\s*\[([^\]]*)\]/)?.[1] ?? "";
      monacoDefinitions.set(
        id,
        [...extensions.matchAll(/["']\.([^"']+)["']/g)]
          .map(([, extension]) => extension.toLowerCase())
          .sort(),
      );
    }
  }
  monacoDefinitions.set("json", ["json"]);

  const rustSource = readFileSync(
    new URL("../../functor_core/src/syntax.rs", import.meta.url),
    "utf8",
  );
  const rustDefinitions = new Map();
  for (const [, id, extensions] of rustSource.matchAll(
    /MonacoLanguageDefinition\s*\{\s*id:\s*"([^"]+)",\s*extensions:\s*&\[([\s\S]*?)\]\s*,?\s*\}/g,
  )) {
    rustDefinitions.set(
      id,
      [...extensions.matchAll(/"([^"]+)"/g)]
        .map(([, extension]) => extension.toLowerCase())
        .sort(),
    );
  }

  const normalize = (definitions) =>
    [...definitions].sort(([left], [right]) => left.localeCompare(right));
  assert.deepEqual(normalize(rustDefinitions), normalize(monacoDefinitions));
});

test("JSON registers only a lexical Monarch tokenizer with Unicode-safe offsets", () => {
  let languageRegistration;
  let providerLanguage;
  let tokenizer;
  registerJsonLanguage({
    languages: {
      getLanguages() {
        return [];
      },
      register(language) {
        languageRegistration = language;
      },
      setMonarchTokensProvider(language, definition) {
        providerLanguage = language;
        tokenizer = definition;
      },
    },
  });

  assert.deepEqual(languageRegistration, {
    id: "json",
    extensions: [".json"],
    aliases: ["JSON"],
  });
  assert.equal(providerLanguage, "json");
  assert.deepEqual(
    tokenizer.tokenizer.root.map(([, token]) => token),
    ["", "string", "string.invalid", "number", "keyword", "delimiter.bracket", "delimiter", "invalid"],
  );
  assert.deepEqual(
    Object.keys(tokenizer).sort(),
    ["defaultToken", "tokenPostfix", "tokenizer"],
  );

  const source = '{\n  "🦀": 42,\n  "ready": true\n}';
  const lines = source.split("\n");
  const line = lines[1];
  const numberRule = jsonMonarchDefinition.tokenizer.root.find(([, token]) => token === "number")[0];
  const number = numberRule.exec(line);
  assert.equal(number?.[0], "42");
  assert.equal(source.indexOf("42"), lines[0].length + 1 + line.indexOf("42"));
});

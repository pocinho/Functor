use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MonacoLanguageDefinition {
    id: &'static str,
    extensions: &'static [&'static str],
}

const MONACO_LANGUAGES: &[MonacoLanguageDefinition] = &[
    MonacoLanguageDefinition {
        id: "abap",
        extensions: &["abap"],
    },
    MonacoLanguageDefinition {
        id: "apex",
        extensions: &["cls"],
    },
    MonacoLanguageDefinition {
        id: "azcli",
        extensions: &["azcli"],
    },
    MonacoLanguageDefinition {
        id: "bat",
        extensions: &["bat", "cmd"],
    },
    MonacoLanguageDefinition {
        id: "bicep",
        extensions: &["bicep"],
    },
    MonacoLanguageDefinition {
        id: "cameligo",
        extensions: &["mligo"],
    },
    MonacoLanguageDefinition {
        id: "clojure",
        extensions: &["clj", "cljs", "cljc", "edn"],
    },
    MonacoLanguageDefinition {
        id: "coffeescript",
        extensions: &["coffee"],
    },
    MonacoLanguageDefinition {
        id: "cpp",
        extensions: &["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
    },
    MonacoLanguageDefinition {
        id: "c",
        extensions: &["c", "h"],
    },
    MonacoLanguageDefinition {
        id: "csharp",
        extensions: &["cs", "csx", "cake"],
    },
    MonacoLanguageDefinition {
        id: "csp",
        extensions: &["csp"],
    },
    MonacoLanguageDefinition {
        id: "css",
        extensions: &["css"],
    },
    MonacoLanguageDefinition {
        id: "cypher",
        extensions: &["cypher", "cyp"],
    },
    MonacoLanguageDefinition {
        id: "dart",
        extensions: &["dart"],
    },
    MonacoLanguageDefinition {
        id: "dockerfile",
        extensions: &["dockerfile"],
    },
    MonacoLanguageDefinition {
        id: "ecl",
        extensions: &["ecl"],
    },
    MonacoLanguageDefinition {
        id: "elixir",
        extensions: &["ex", "exs"],
    },
    MonacoLanguageDefinition {
        id: "flow9",
        extensions: &["flow"],
    },
    MonacoLanguageDefinition {
        id: "freemarker2",
        extensions: &["ftl", "ftlh", "ftlx"],
    },
    MonacoLanguageDefinition {
        id: "freemarker2.tag-angle.interpolation-bracket",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "freemarker2.tag-angle.interpolation-dollar",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "freemarker2.tag-auto.interpolation-bracket",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "freemarker2.tag-auto.interpolation-dollar",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "freemarker2.tag-bracket.interpolation-bracket",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "freemarker2.tag-bracket.interpolation-dollar",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "fsharp",
        extensions: &["fs", "fsi", "ml", "mli", "fsx", "fsscript"],
    },
    MonacoLanguageDefinition {
        id: "go",
        extensions: &["go"],
    },
    MonacoLanguageDefinition {
        id: "graphql",
        extensions: &["graphql", "gql"],
    },
    MonacoLanguageDefinition {
        id: "handlebars",
        extensions: &["handlebars", "hbs"],
    },
    MonacoLanguageDefinition {
        id: "hcl",
        extensions: &["tf", "tfvars", "hcl"],
    },
    MonacoLanguageDefinition {
        id: "html",
        extensions: &[
            "html", "htm", "shtml", "xhtml", "mdoc", "jsp", "asp", "aspx", "jshtm",
        ],
    },
    MonacoLanguageDefinition {
        id: "ini",
        extensions: &["ini", "properties", "gitconfig"],
    },
    MonacoLanguageDefinition {
        id: "java",
        extensions: &["java", "jav"],
    },
    MonacoLanguageDefinition {
        id: "javascript",
        extensions: &["js", "es6", "jsx", "mjs", "cjs"],
    },
    MonacoLanguageDefinition {
        id: "julia",
        extensions: &["jl"],
    },
    MonacoLanguageDefinition {
        id: "kotlin",
        extensions: &["kt", "kts"],
    },
    MonacoLanguageDefinition {
        id: "less",
        extensions: &["less"],
    },
    MonacoLanguageDefinition {
        id: "lexon",
        extensions: &["lex"],
    },
    MonacoLanguageDefinition {
        id: "liquid",
        extensions: &["liquid", "html.liquid"],
    },
    MonacoLanguageDefinition {
        id: "lua",
        extensions: &["lua"],
    },
    MonacoLanguageDefinition {
        id: "m3",
        extensions: &["m3", "i3", "mg", "ig"],
    },
    MonacoLanguageDefinition {
        id: "markdown",
        extensions: &[
            "md", "markdown", "mdown", "mkdn", "mkd", "mdwn", "mdtxt", "mdtext",
        ],
    },
    MonacoLanguageDefinition {
        id: "mdx",
        extensions: &["mdx"],
    },
    MonacoLanguageDefinition {
        id: "mips",
        extensions: &["s"],
    },
    MonacoLanguageDefinition {
        id: "msdax",
        extensions: &["dax", "msdax"],
    },
    MonacoLanguageDefinition {
        id: "mysql",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "objective-c",
        extensions: &["m"],
    },
    MonacoLanguageDefinition {
        id: "pascal",
        extensions: &["pas", "p", "pp"],
    },
    MonacoLanguageDefinition {
        id: "pascaligo",
        extensions: &["ligo"],
    },
    MonacoLanguageDefinition {
        id: "perl",
        extensions: &["pl", "pm"],
    },
    MonacoLanguageDefinition {
        id: "pgsql",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "php",
        extensions: &["php", "php4", "php5", "phtml", "ctp"],
    },
    MonacoLanguageDefinition {
        id: "pla",
        extensions: &["pla"],
    },
    MonacoLanguageDefinition {
        id: "postiats",
        extensions: &["dats", "sats", "hats"],
    },
    MonacoLanguageDefinition {
        id: "powerquery",
        extensions: &["pq", "pqm"],
    },
    MonacoLanguageDefinition {
        id: "powershell",
        extensions: &["ps1", "psm1", "psd1"],
    },
    MonacoLanguageDefinition {
        id: "proto",
        extensions: &["proto"],
    },
    MonacoLanguageDefinition {
        id: "pug",
        extensions: &["jade", "pug"],
    },
    MonacoLanguageDefinition {
        id: "python",
        extensions: &["py", "rpy", "pyw", "cpy", "gyp", "gypi"],
    },
    MonacoLanguageDefinition {
        id: "qsharp",
        extensions: &["qs"],
    },
    MonacoLanguageDefinition {
        id: "r",
        extensions: &["r", "rhistory", "rmd", "rprofile", "rt"],
    },
    MonacoLanguageDefinition {
        id: "razor",
        extensions: &["cshtml"],
    },
    MonacoLanguageDefinition {
        id: "redis",
        extensions: &["redis"],
    },
    MonacoLanguageDefinition {
        id: "redshift",
        extensions: &[],
    },
    MonacoLanguageDefinition {
        id: "restructuredtext",
        extensions: &["rst"],
    },
    MonacoLanguageDefinition {
        id: "ruby",
        extensions: &["rb", "rbx", "rjs", "gemspec", "pp"],
    },
    MonacoLanguageDefinition {
        id: "rust",
        extensions: &["rs", "rlib"],
    },
    MonacoLanguageDefinition {
        id: "sb",
        extensions: &["sb"],
    },
    MonacoLanguageDefinition {
        id: "scala",
        extensions: &["scala", "sc", "sbt"],
    },
    MonacoLanguageDefinition {
        id: "scheme",
        extensions: &["scm", "ss", "sch", "rkt"],
    },
    MonacoLanguageDefinition {
        id: "scss",
        extensions: &["scss"],
    },
    MonacoLanguageDefinition {
        id: "shell",
        extensions: &["sh", "bash"],
    },
    MonacoLanguageDefinition {
        id: "sol",
        extensions: &["sol"],
    },
    MonacoLanguageDefinition {
        id: "aes",
        extensions: &["aes"],
    },
    MonacoLanguageDefinition {
        id: "sparql",
        extensions: &["rq"],
    },
    MonacoLanguageDefinition {
        id: "sql",
        extensions: &["sql"],
    },
    MonacoLanguageDefinition {
        id: "st",
        extensions: &[
            "st", "iecst", "iecplc", "lc3lib", "tcpou", "tcdut", "tcgvl", "tcio",
        ],
    },
    MonacoLanguageDefinition {
        id: "swift",
        extensions: &["swift"],
    },
    MonacoLanguageDefinition {
        id: "systemverilog",
        extensions: &["sv", "svh"],
    },
    MonacoLanguageDefinition {
        id: "tcl",
        extensions: &["tcl"],
    },
    MonacoLanguageDefinition {
        id: "twig",
        extensions: &["twig"],
    },
    MonacoLanguageDefinition {
        id: "typescript",
        extensions: &["ts", "tsx", "cts", "mts"],
    },
    MonacoLanguageDefinition {
        id: "typespec",
        extensions: &["tsp"],
    },
    MonacoLanguageDefinition {
        id: "vb",
        extensions: &["vb"],
    },
    MonacoLanguageDefinition {
        id: "verilog",
        extensions: &["v", "vh"],
    },
    MonacoLanguageDefinition {
        id: "wgsl",
        extensions: &["wgsl"],
    },
    MonacoLanguageDefinition {
        id: "xml",
        extensions: &[
            "xml", "xsd", "dtd", "ascx", "csproj", "config", "props", "targets", "wxi", "wxl",
            "wxs", "xaml", "svg", "svgz", "opf", "xslt", "xsl",
        ],
    },
    MonacoLanguageDefinition {
        id: "yaml",
        extensions: &["yaml", "yml"],
    },
    MonacoLanguageDefinition {
        id: "json",
        extensions: &["json"],
    },
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyntaxLanguage {
    #[default]
    Auto,
    PlainText,
    Rust,
    CSharp,
    FSharp,
    Json,
    Markdown,
    Monaco(&'static str),
}

impl SyntaxLanguage {
    pub fn from_extension(extension: Option<&str>) -> Self {
        let Some(extension) = extension else {
            return Self::Auto;
        };
        let extension = extension.trim_start_matches('.').to_ascii_lowercase();
        MONACO_LANGUAGES
            .iter()
            .find(|language| language.extensions.contains(&extension.as_str()))
            .and_then(|language| Self::from_monaco_id(language.id))
            .unwrap_or(Self::PlainText)
    }

    pub fn from_path(path: &Path) -> Self {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("dockerfile"))
        {
            return Self::Monaco("dockerfile");
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        Self::from_extension(extension.as_deref())
    }

    pub fn from_monaco_id(id: &str) -> Option<Self> {
        match id {
            "plainText" | "plaintext" => Some(Self::PlainText),
            "rust" => Some(Self::Rust),
            "csharp" => Some(Self::CSharp),
            "fsharp" => Some(Self::FSharp),
            "json" => Some(Self::Json),
            "markdown" => Some(Self::Markdown),
            _ => MONACO_LANGUAGES
                .iter()
                .find(|language| language.id == id)
                .map(|language| Self::Monaco(language.id)),
        }
    }

    pub fn monaco_id(self) -> &'static str {
        match self {
            Self::Auto | Self::PlainText => "plainText",
            Self::Rust => "rust",
            Self::CSharp => "csharp",
            Self::FSharp => "fsharp",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Monaco(id) => id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighlightKind {
    Keyword,
    String,
    Comment,
    Number,
    Punctuation,
    Heading,
    Emphasis,
    Code,
    Link,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightKind,
}

pub fn highlight_line(text: &str, language: SyntaxLanguage) -> Vec<HighlightSpan> {
    match language {
        SyntaxLanguage::Auto => highlight_line(text, detect_language(text)),
        SyntaxLanguage::PlainText => Vec::new(),
        SyntaxLanguage::Rust => highlight_rust(text),
        SyntaxLanguage::CSharp | SyntaxLanguage::FSharp | SyntaxLanguage::Json => Vec::new(),
        SyntaxLanguage::Markdown => highlight_markdown(text),
        SyntaxLanguage::Monaco(_) => Vec::new(),
    }
}

fn detect_language(text: &str) -> SyntaxLanguage {
    let trimmed = text.trim_start();
    if trimmed.starts_with('#') || text.contains('`') || text.contains("](") {
        SyntaxLanguage::Markdown
    } else if text
        .split(|character: char| !is_identifier_continue(character))
        .any(|identifier| {
            !identifier.is_empty() && is_rust_keyword(&identifier.chars().collect::<Vec<_>>())
        })
    {
        SyntaxLanguage::Rust
    } else {
        SyntaxLanguage::PlainText
    }
}

fn highlight_rust(text: &str) -> Vec<HighlightSpan> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut spans = Vec::new();
    let mut index = 0;

    while index < characters.len() {
        if characters[index] == '/' && characters.get(index + 1) == Some(&'/') {
            spans.push(HighlightSpan {
                start: index,
                end: characters.len(),
                kind: HighlightKind::Comment,
            });
            break;
        }

        if characters[index] == '"' {
            let start = index;
            index += 1;
            while index < characters.len() {
                let escaped = index > start && characters[index - 1] == '\\';
                if characters[index] == '"' && !escaped {
                    index += 1;
                    break;
                }
                index += 1;
            }
            spans.push(HighlightSpan {
                start,
                end: index,
                kind: HighlightKind::String,
            });
            continue;
        }

        if characters[index].is_ascii_digit() {
            let start = index;
            index += 1;
            while index < characters.len()
                && (characters[index].is_ascii_digit() || characters[index] == '.')
            {
                index += 1;
            }
            spans.push(HighlightSpan {
                start,
                end: index,
                kind: HighlightKind::Number,
            });
            continue;
        }

        if is_identifier_start(characters[index]) {
            let start = index;
            index += 1;
            while index < characters.len() && is_identifier_continue(characters[index]) {
                index += 1;
            }
            if is_rust_keyword(&characters[start..index]) {
                spans.push(HighlightSpan {
                    start,
                    end: index,
                    kind: HighlightKind::Keyword,
                });
            }
            continue;
        }

        if characters[index].is_ascii_punctuation() && characters[index] != '_' {
            spans.push(HighlightSpan {
                start: index,
                end: index + 1,
                kind: HighlightKind::Punctuation,
            });
        }
        index += 1;
    }

    spans
}

fn highlight_markdown(text: &str) -> Vec<HighlightSpan> {
    let characters = text.chars().collect::<Vec<_>>();
    let trimmed_start = characters
        .iter()
        .position(|character| !character.is_whitespace())
        .unwrap_or(characters.len());
    let mut spans = Vec::new();

    if characters[trimmed_start..].starts_with(&['#']) {
        spans.push(HighlightSpan {
            start: trimmed_start,
            end: characters.len(),
            kind: HighlightKind::Heading,
        });
        return spans;
    }

    let mut index = 0;
    while index < characters.len() {
        if characters[index] == '`' {
            let start = index;
            index += 1;
            while index < characters.len() && characters[index] != '`' {
                index += 1;
            }
            if index < characters.len() {
                index += 1;
            }
            spans.push(HighlightSpan {
                start,
                end: index,
                kind: HighlightKind::Code,
            });
        } else if characters[index] == '[' {
            if let Some(close) = characters[index + 1..]
                .iter()
                .position(|character| *character == ']')
            {
                let close = index + 1 + close;
                if characters.get(close + 1) == Some(&'(')
                    && let Some(end) = characters[close + 2..]
                        .iter()
                        .position(|character| *character == ')')
                {
                    spans.push(HighlightSpan {
                        start: index,
                        end: close + 3 + end,
                        kind: HighlightKind::Link,
                    });
                    index = close + 3 + end;
                    continue;
                }
            }
            index += 1;
        } else if characters[index] == '*' || characters[index] == '_' {
            let marker = characters[index];
            let start = index;
            index += 1;
            while index < characters.len() && characters[index] != marker {
                index += 1;
            }
            if index < characters.len() {
                index += 1;
                spans.push(HighlightSpan {
                    start,
                    end: index,
                    kind: HighlightKind::Emphasis,
                });
            }
        } else {
            index += 1;
        }
    }

    spans
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn is_rust_keyword(identifier: &[char]) -> bool {
    matches!(
        identifier,
        ['a', 's']
            | ['a', 's', 'y', 'n', 'c']
            | ['a', 'w', 'a', 'i', 't']
            | ['b', 'r', 'e', 'a', 'k']
            | ['c', 'o', 'n', 's', 't']
            | ['c', 'r', 'a', 't', 'e']
            | ['d', 'y', 'n']
            | ['e', 'l', 's', 'e']
            | ['e', 'n', 'u', 'm']
            | ['e', 'x', 't', 'e', 'r', 'n']
            | ['f', 'n']
            | ['f', 'o', 'r']
            | ['i', 'f']
            | ['i', 'm', 'p', 'l']
            | ['i', 'n']
            | ['l', 'e', 't']
            | ['l', 'o', 'o', 'p']
            | ['m', 'a', 't', 'c', 'h']
            | ['m', 'o', 'd']
            | ['m', 'o', 'v', 'e']
            | ['m', 'u', 't']
            | ['p', 'u', 'b']
            | ['r', 'e', 'f']
            | ['r', 'e', 't', 'u', 'r', 'n']
            | ['s', 't', 'r', 'u', 'c', 't']
            | ['s', 'u', 'p', 'e', 'r']
            | ['t', 'r', 'a', 'i', 't']
            | ['t', 'y', 'p', 'e']
            | ['u', 'n', 's', 'a', 'f', 'e']
            | ['u', 's', 'e']
            | ['w', 'h', 'e', 'r', 'e']
            | ['w', 'h', 'i', 'l', 'e']
    )
}

#[cfg(test)]
mod tests {
    use super::{HighlightKind, MONACO_LANGUAGES, SyntaxLanguage, highlight_line};
    use std::path::Path;

    #[test]
    fn paths_select_supported_source_languages_case_insensitively() {
        assert_eq!(
            SyntaxLanguage::from_path(Path::new("Program.CS")),
            SyntaxLanguage::CSharp
        );
        assert_eq!(
            SyntaxLanguage::from_path(Path::new("Library.FSX")),
            SyntaxLanguage::FSharp
        );
        assert_eq!(
            SyntaxLanguage::from_path(Path::new("settings.JSON")),
            SyntaxLanguage::Json
        );
        assert_eq!(
            SyntaxLanguage::from_path(Path::new("guide.Markdown")),
            SyntaxLanguage::Markdown
        );
    }

    #[test]
    fn every_registered_monaco_language_id_round_trips() {
        for definition in MONACO_LANGUAGES {
            let language = SyntaxLanguage::from_monaco_id(definition.id).unwrap();
            assert_eq!(language.monaco_id(), definition.id);
        }
        assert!(SyntaxLanguage::from_monaco_id("unknown-language").is_none());
    }

    #[test]
    fn paths_detect_languages_from_the_expanded_monaco_catalogue() {
        assert_eq!(
            SyntaxLanguage::from_path(Path::new("Dockerfile")),
            SyntaxLanguage::Monaco("dockerfile")
        );
        assert_eq!(
            SyntaxLanguage::from_path(Path::new("app.CPP")),
            SyntaxLanguage::Monaco("cpp")
        );
        assert_eq!(
            SyntaxLanguage::from_path(Path::new("module.V")),
            SyntaxLanguage::Monaco("verilog")
        );
    }

    #[test]
    fn rust_highlighting_marks_keywords_strings_numbers_and_comments() {
        let spans = highlight_line("fn main() { let value = 42; // note", SyntaxLanguage::Rust);

        assert!(spans.iter().any(|span| span.kind == HighlightKind::Keyword));
        assert!(spans.iter().any(|span| span.kind == HighlightKind::Number));
        assert!(spans.iter().any(|span| span.kind == HighlightKind::Comment));
    }

    #[test]
    fn markdown_highlighting_marks_headings_code_and_links() {
        assert_eq!(
            highlight_line("# Title", SyntaxLanguage::Markdown)[0].kind,
            HighlightKind::Heading
        );
        let spans = highlight_line(
            "See `code` [link](https://example.com)",
            SyntaxLanguage::Markdown,
        );
        assert!(spans.iter().any(|span| span.kind == HighlightKind::Code));
        assert!(spans.iter().any(|span| span.kind == HighlightKind::Link));
    }

    #[test]
    fn plain_text_has_no_highlight_spans() {
        assert!(highlight_line("plain", SyntaxLanguage::PlainText).is_empty());
    }

    #[test]
    fn auto_language_detects_rust_and_markdown_examples() {
        assert!(!highlight_line("fn main() {}", SyntaxLanguage::Auto).is_empty());
        assert!(!highlight_line("# Heading", SyntaxLanguage::Auto).is_empty());
        assert!(highlight_line("ordinary prose", SyntaxLanguage::Auto).is_empty());
    }
}

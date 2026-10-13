//! The rule catalog: stable codes, severities and one-line descriptions.
//!
//! Codes are a public contract and are never reused. This table and the one in
//! `docs/design/validation-rules.md` must agree; a test enforces it.

use functor_model::{RuleCode, Severity};

/// What a rule checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleInfo {
    /// The stable code.
    pub code: RuleCode,
    /// The severity of every finding of this rule.
    pub severity: Severity,
    /// A one-line description.
    pub title: &'static str,
}

macro_rules! rules {
    ($($name:ident = $code:literal, $severity:ident, $title:literal;)*) => {
        $(
            #[doc = $title]
            pub const $name: RuleCode = RuleCode::new($code);
        )*

        /// Every rule, ordered by code.
        pub const CATALOG: &[RuleInfo] = &[
            $(RuleInfo { code: $name, severity: Severity::$severity, title: $title },)*
        ];
    };
}

rules! {
    V0001 = "V0001", Error, "Entity file is malformed";
    V0002 = "V0002", Error, "Missing identifier";
    V0003 = "V0003", Error, "Duplicate artifact id";
    V0004 = "V0004", Error, "Duplicate artifact slug";
    V0005 = "V0005", Error, "Invalid identifier or slug format";
    V0006 = "V0006", Error, "Relationship source does not exist";
    V0007 = "V0007", Error, "Relationship target does not exist";
    V0008 = "V0008", Error, "Duplicate relationship id";
    V0009 = "V0009", Error, "Self-referencing relationship of an acyclic type";
    V0010 = "V0010", Error, "Content path is unsafe or the referenced file does not exist";
    V0011 = "V0011", Error, "Cycle in an acyclic relationship type";
    V0012 = "V0012", Warning, "Custom relationship type in use";
    V0013 = "V0013", Error, "Unsupported format version";
    V0014 = "V0014", Error, "Repository structure violation";
    V0015 = "V0015", Warning, "Duplicate relationship";
    V0016 = "V0016", Warning, "Unexpected file in the entity directories";
    V0017 = "V0017", Error, "File name does not match the entity id";
    V0018 = "V0018", Info, "Unknown field";
    V0019 = "V0019", Info, "File or directory could not be read";
}

/// Looks up a rule by code.
#[must_use]
pub fn rule_info(code: RuleCode) -> Option<&'static RuleInfo> {
    CATALOG.iter().find(|info| info.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_ordered_unique_and_complete() {
        let codes: Vec<&str> = CATALOG.iter().map(|r| r.code.as_str()).collect();
        let expected: Vec<String> = (1..=19).map(|n| format!("V{n:04}")).collect();
        assert_eq!(codes, expected);
    }

    #[test]
    fn rule_info_finds_every_rule() {
        for info in CATALOG {
            assert_eq!(rule_info(info.code), Some(info));
        }
        assert_eq!(rule_info(RuleCode::new("V9999")), None);
    }

    /// The catalog table of `docs/design/validation-rules.md` lists the same codes and severities.
    #[test]
    fn the_catalog_matches_the_documentation() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/design/validation-rules.md"
        );
        let text = std::fs::read_to_string(path).unwrap();
        let documented: Vec<(String, String)> = text
            .lines()
            .filter_map(|line| {
                let cells: Vec<&str> = line.split('|').map(str::trim).collect();
                let code = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
                (code.len() == 5 && code.starts_with('V')).then(|| {
                    (
                        code.to_owned(),
                        cells.get(2).copied().unwrap_or_default().to_owned(),
                    )
                })
            })
            .collect();
        let implemented: Vec<(String, String)> = CATALOG
            .iter()
            .map(|r| {
                let severity = match r.severity {
                    Severity::Error => "Error",
                    Severity::Warning => "Warning",
                    Severity::Info => "Info",
                    _ => panic!("unknown severity"),
                };
                (r.code.as_str().to_owned(), severity.to_owned())
            })
            .collect();
        assert_eq!(documented, implemented);
    }
}

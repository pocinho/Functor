use functor_core::syntax::SyntaxLanguage;
use serde::{Deserialize, Serialize};

pub const MAX_DOCUMENT_EDITS: usize = 256;
pub const MAX_INSERTED_TEXT_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenDocumentRequest {}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenWorkspaceRequest {}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceDirectoryRequest {
    pub relative_path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceFileRequest {
    pub relative_path: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchWorkspaceRequest {
    pub search_id: String,
    pub query: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelSearchRequest {
    pub search_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenWorkspaceSearchResultRequest {
    pub relative_path: String,
    pub line_number: u32,
    pub column: u32,
    pub match_length: u32,
    pub document_id: Option<u64>,
    pub expected_revision: Option<u64>,
    pub source_fingerprint: Option<String>,
    pub workspace_root: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceWorkspaceSearchResultsRequest {
    pub workspace_root: String,
    pub replacement: String,
    pub files: Vec<WorkspaceSearchFileReplacement>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSearchFileReplacement {
    pub relative_path: String,
    pub source_fingerprint: String,
    pub matches: Vec<WorkspaceSearchMatchReplacement>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSearchMatchReplacement {
    pub range_offset: u32,
    pub range_length: u32,
    pub matched_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentLanguage(SyntaxLanguage);

#[allow(non_upper_case_globals)]
impl DocumentLanguage {
    pub const Rust: Self = Self(SyntaxLanguage::Rust);
    pub const CSharp: Self = Self(SyntaxLanguage::CSharp);
    pub const FSharp: Self = Self(SyntaxLanguage::FSharp);
    pub const Json: Self = Self(SyntaxLanguage::Json);
    pub const Markdown: Self = Self(SyntaxLanguage::Markdown);
    pub const PlainText: Self = Self(SyntaxLanguage::PlainText);

    pub fn from_syntax(language: SyntaxLanguage) -> Self {
        match language {
            SyntaxLanguage::Auto => Self::PlainText,
            _ => Self(language),
        }
    }

    pub fn into_syntax(self) -> SyntaxLanguage {
        self.0
    }
}

impl Serialize for DocumentLanguage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.monaco_id())
    }
}

impl<'de> Deserialize<'de> for DocumentLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        SyntaxLanguage::from_monaco_id(&id)
            .map(Self::from_syntax)
            .ok_or_else(|| {
                serde::de::Error::custom(format!("Unsupported Monaco language identifier `{id}`."))
            })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewDocumentRequest {
    pub language: DocumentLanguage,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwitchDocumentRequest {
    pub view_id: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloseDocumentRequest {
    pub view_id: u64,
    pub expected_revision: u64,
    pub discard_changes: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveDocumentRequest {
    pub document_id: u64,
    pub expected_revision: u64,
    #[serde(default)]
    pub save_as: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyDocumentEditsRequest {
    pub document_id: u64,
    pub expected_revision: u64,
    pub changes: Vec<MonacoTextChange>,
    pub is_undoing: bool,
    pub is_redoing: bool,
    pub cursor: MonacoPosition,
    pub selection: Option<MonacoSelection>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSelectionRequest {
    pub document_id: u64,
    pub expected_revision: u64,
    pub cursor: MonacoPosition,
    pub selection: Option<MonacoSelection>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetDocumentLanguageRequest {
    pub document_id: u64,
    pub expected_revision: u64,
    pub language: DocumentLanguage,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonacoTextChange {
    pub range_offset: u32,
    pub range_length: u32,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonacoPosition {
    pub line_number: u32,
    pub column: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonacoSelection {
    pub anchor: MonacoPosition,
    pub focus: MonacoPosition,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub documents: Vec<DocumentSnapshot>,
    pub active_view_id: u64,
    pub workspace: Option<WorkspaceStateSnapshot>,
    pub workspace_error: Option<String>,
    pub persistence_warning: Option<String>,
    pub can_reopen_closed_document: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreWorkspaceResponse {
    pub snapshot: WorkspaceSnapshot,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveRecoverySnapshotRequest {
    pub document_id: u64,
    pub expected_revision: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveRecoverySnapshotResponse {
    pub saved_revision: Option<u64>,
    pub stale: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCandidate {
    pub recovery_id: String,
    pub title: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCandidatesResponse {
    pub candidates: Vec<RecoveryCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRestoreResponse {
    pub snapshot: WorkspaceSnapshot,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStateSnapshot {
    pub root: String,
    pub name: String,
    pub entries: Vec<WorkspaceEntrySnapshot>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceEntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntrySnapshot {
    pub name: String,
    pub relative_path: String,
    pub kind: WorkspaceEntryKind,
    pub children: Option<Vec<WorkspaceEntrySnapshot>>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchHit {
    pub relative_path: String,
    pub document_id: Option<u64>,
    pub revision: Option<u64>,
    pub line_number: u32,
    pub column: u32,
    pub match_length: u32,
    pub range_offset: u32,
    pub range_length: u32,
    pub matched_text: String,
    pub source_fingerprint: Option<String>,
    pub workspace_root: Option<String>,
    pub preview: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchResponse {
    pub search_id: String,
    pub workspace_root: Option<String>,
    pub hits: Vec<WorkspaceSearchHit>,
    pub cancelled: bool,
    pub skipped_files: usize,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceWorkspaceSearchResultsResponse {
    pub replaced: usize,
    pub files_written: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreset {
    GraphiteDark,
    GraphiteLight,
    Custom,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsColors {
    pub background: String,
    pub foreground: String,
    pub selection: String,
    pub cursor: String,
    pub line_number: String,
    pub gutter_background: String,
    pub gutter_separator: String,
    pub editor_border: String,
    pub diagnostic_error: String,
    pub diagnostic_warning: String,
    pub diagnostic_info: String,
    pub syntax_keyword: String,
    pub syntax_string: String,
    pub syntax_comment: String,
    pub syntax_number: String,
    pub syntax_type: String,
    pub syntax_function: String,
    pub resize_handle_color: String,
    pub command_palette_shadow_color: String,
    pub workspace_separator_color: String,
    pub measurement_color: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsTypography {
    pub editor_font_family: String,
    pub editor_fallback_font_family: String,
    pub ui_font_family: String,
    pub workspace_font_size: f64,
    pub editor_font_size: f64,
    pub editor_line_height: f64,
    pub editor_tab_size: u32,
    pub command_palette_font_size: f64,
    pub welcome_title_font_size: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsGeometry {
    pub shell_padding_x: f64,
    pub shell_padding_y: f64,
    pub workspace_sidebar_width: f64,
    pub editor_border_width: f64,
    pub document_tab_min_height: f64,
    pub document_tab_padding_horizontal: f64,
    pub document_tab_padding_vertical: f64,
    pub command_palette_width: f64,
    pub command_palette_padding: f64,
    pub command_palette_max_height: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub schema_version: u32,
    pub preset: ThemePreset,
    pub colors: SettingsColors,
    pub typography: SettingsTypography,
    pub geometry: SettingsGeometry,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveSettingsRequest {
    pub settings: AppSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserThemeRequest {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveUserThemeRequest {
    pub name: String,
    pub settings: AppSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserThemeSummary {
    pub name: String,
    pub preset: ThemePreset,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsCatalogResponse {
    pub settings: AppSettings,
    pub presets: Vec<AppSettings>,
    pub user_themes: Vec<UserThemeSummary>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSnapshot {
    pub document_id: u64,
    pub view_id: u64,
    pub title: String,
    pub is_untitled: bool,
    pub text: String,
    pub language: DocumentLanguage,
    pub revision: u64,
    pub dirty: bool,
    pub cursor: MonacoPosition,
    pub selection: Option<MonacoSelection>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplyDocumentEditsResponse {
    pub document_id: u64,
    pub revision: u64,
    pub dirty: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSelectionResponse {
    pub document_id: u64,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BridgeErrorCode {
    InvalidRequest,
    DocumentNotFound,
    StaleRevision,
    Cancelled,
    FileIo,
    Internal,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    pub code: BridgeErrorCode,
    pub message: String,
}

impl BridgeError {
    pub fn new(code: BridgeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(BridgeErrorCode::InvalidRequest, message)
    }
}

pub fn validate_text_changes(
    changes: &[MonacoTextChange],
    source: &str,
) -> Result<(), BridgeError> {
    if changes.is_empty() || changes.len() > MAX_DOCUMENT_EDITS {
        return Err(BridgeError::invalid_request(
            "An edit transaction must contain between 1 and 256 changes.",
        ));
    }

    let source_length = source.encode_utf16().count();
    let mut ranges = Vec::with_capacity(changes.len());
    let mut inserted_bytes = 0usize;
    for change in changes {
        let start = change.range_offset as usize;
        let end = start
            .checked_add(change.range_length as usize)
            .ok_or_else(|| BridgeError::invalid_request("An edit range overflowed."))?;
        if end > source_length
            || !is_utf16_boundary(source, start)
            || !is_utf16_boundary(source, end)
        {
            return Err(BridgeError::invalid_request(
                "An edit range is outside the current document or splits a Unicode character.",
            ));
        }
        inserted_bytes = inserted_bytes
            .checked_add(change.text.len())
            .ok_or_else(|| BridgeError::invalid_request("Inserted text is too large."))?;
        ranges.push((start, end));
    }

    if inserted_bytes > MAX_INSERTED_TEXT_BYTES {
        return Err(BridgeError::invalid_request(
            "An edit transaction exceeds the 1 MiB insertion limit.",
        ));
    }

    ranges.sort_unstable();
    if ranges
        .windows(2)
        .any(|pair| pair[1].0 < pair[0].1 || pair[1].0 == pair[0].0)
    {
        return Err(BridgeError::invalid_request(
            "Edit ranges must not overlap or share an insertion position.",
        ));
    }

    Ok(())
}

fn is_utf16_boundary(text: &str, target: usize) -> bool {
    let mut offset = 0usize;
    if target == 0 {
        return true;
    }
    for character in text.chars() {
        offset += character.len_utf16();
        if offset == target {
            return true;
        }
        if offset > target {
            return false;
        }
    }
    offset == target
}

#[cfg(test)]
mod tests {
    use super::{
        ApplyDocumentEditsRequest, BridgeErrorCode, DocumentLanguage, MonacoTextChange,
        SaveDocumentRequest, SetDocumentLanguageRequest, validate_text_changes,
    };

    #[test]
    fn document_languages_round_trip_with_stable_frontend_ids() {
        for language_id in ["csharp", "fsharp", "json", "typescript", "coffeescript"] {
            let language =
                serde_json::from_str::<DocumentLanguage>(&format!("\"{language_id}\"")).unwrap();
            assert_eq!(
                serde_json::to_string(&language).unwrap(),
                format!("\"{language_id}\"")
            );
        }
        assert_eq!(
            serde_json::to_string(&DocumentLanguage::PlainText).unwrap(),
            "\"plainText\""
        );
        assert!(serde_json::from_str::<DocumentLanguage>("\"unknown\"").is_err());
    }

    #[test]
    fn set_language_request_rejects_unknown_fields_and_language_ids() {
        for request in [
            r#"{"documentId":1,"expectedRevision":0,"language":"unknown"}"#,
            r#"{"documentId":1,"expectedRevision":0,"language":"rust","path":"C:\\secret"}"#,
        ] {
            assert!(serde_json::from_str::<SetDocumentLanguageRequest>(request).is_err());
        }
    }

    #[test]
    fn rejects_unknown_request_fields() {
        let result = serde_json::from_str::<ApplyDocumentEditsRequest>(
            r#"{"documentId":1,"expectedRevision":0,"changes":[],"isUndoing":false,"isRedoing":false,"cursor":{"lineNumber":1,"column":1},"selection":null,"path":"C:\\secret"}"#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn save_as_request_defaults_to_regular_save_and_accepts_explicit_save_as() {
        let regular =
            serde_json::from_str::<SaveDocumentRequest>(r#"{"documentId":1,"expectedRevision":0}"#)
                .unwrap();
        let save_as = serde_json::from_str::<SaveDocumentRequest>(
            r#"{"documentId":1,"expectedRevision":0,"saveAs":true}"#,
        )
        .unwrap();

        assert!(!regular.save_as);
        assert!(save_as.save_as);
    }

    #[test]
    fn rejects_stale_or_split_unicode_ranges_at_the_protocol_boundary() {
        let result = validate_text_changes(
            &[MonacoTextChange {
                range_offset: 1,
                range_length: 0,
                text: "x".into(),
            }],
            "🦀",
        );

        assert_eq!(result.unwrap_err().code, BridgeErrorCode::InvalidRequest);
    }

    #[test]
    fn rejects_overlapping_ranges_and_duplicate_insertions() {
        for changes in [
            vec![
                MonacoTextChange {
                    range_offset: 0,
                    range_length: 2,
                    text: String::new(),
                },
                MonacoTextChange {
                    range_offset: 1,
                    range_length: 1,
                    text: String::new(),
                },
            ],
            vec![
                MonacoTextChange {
                    range_offset: 1,
                    range_length: 0,
                    text: "a".into(),
                },
                MonacoTextChange {
                    range_offset: 1,
                    range_length: 0,
                    text: "b".into(),
                },
            ],
        ] {
            assert_eq!(
                validate_text_changes(&changes, "abcd").unwrap_err().code,
                BridgeErrorCode::InvalidRequest
            );
        }
    }

    #[test]
    fn accepts_adjacent_non_overlapping_changes() {
        let changes = [
            MonacoTextChange {
                range_offset: 0,
                range_length: 1,
                text: "A".into(),
            },
            MonacoTextChange {
                range_offset: 1,
                range_length: 1,
                text: "B".into(),
            },
        ];

        assert!(validate_text_changes(&changes, "ab").is_ok());
    }
}

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::protocol::{
    AppSettings, SettingsCatalogResponse, SettingsColors, SettingsGeometry, SettingsTypography,
    ThemePreset, UserThemeSummary,
};

const SETTINGS_SCHEMA_VERSION: u32 = 1;
const SETTINGS_FILE_NAME: &str = "settings.json";
const THEMES_DIRECTORY_NAME: &str = "themes";
const THEME_FILE_EXTENSION: &str = "functortheme";
const MAX_SETTINGS_BYTES: usize = 1_048_576;
const MAX_USER_THEMES: usize = 100;
const MAX_THEME_FILES_SCANNED: usize = 500;
const MAX_THEME_NAME_CHARS: usize = 64;
const MAX_THEME_WARNINGS: usize = 20;
static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(1);

pub fn defaults() -> AppSettings {
    for_preset(ThemePreset::GraphiteDark)
}

pub fn for_preset(preset: ThemePreset) -> AppSettings {
    let light = preset == ThemePreset::GraphiteLight;
    let (
        background,
        foreground,
        selection,
        cursor,
        line_number,
        gutter_background,
        separator,
        error,
        warning,
        info,
    ) = if light {
        (
            "#FFE7E5EA",
            "#FF24212B",
            "#604E3A78",
            "#FF4E3A78",
            "#FF77707F",
            "#FFDCD9E1",
            "#FFB9B3C2",
            "#FFB83B5E",
            "#FF9A6B18",
            "#FF356B8C",
        )
    } else {
        (
            "#FF17151B",
            "#FFE7E3ED",
            "#705E4A8F",
            "#FFD8C9F0",
            "#FF8F8799",
            "#FF1E1B24",
            "#FF393241",
            "#FFFF7185",
            "#FFE4B45D",
            "#FF72C5E8",
        )
    };
    let (
        syntax_keyword,
        syntax_string,
        syntax_comment,
        syntax_number,
        syntax_type,
        syntax_function,
    ) = if light {
        (
            "#FF633F9A",
            "#FF8C4B4B",
            "#FF59705A",
            "#FF3B6871",
            "#FF356B8C",
            "#FF7A4F8C",
        )
    } else {
        (
            "#FFC7A6F7",
            "#FFE5A58F",
            "#FF7CA889",
            "#FF9DD6C8",
            "#FF7CC4C7",
            "#FFE1B1E8",
        )
    };
    AppSettings {
        schema_version: SETTINGS_SCHEMA_VERSION,
        preset,
        colors: SettingsColors {
            background: background.into(),
            foreground: foreground.into(),
            selection: selection.into(),
            cursor: cursor.into(),
            line_number: line_number.into(),
            gutter_background: gutter_background.into(),
            gutter_separator: separator.into(),
            editor_border: separator.into(),
            diagnostic_error: error.into(),
            diagnostic_warning: warning.into(),
            diagnostic_info: info.into(),
            syntax_keyword: syntax_keyword.into(),
            syntax_string: syntax_string.into(),
            syntax_comment: syntax_comment.into(),
            syntax_number: syntax_number.into(),
            syntax_type: syntax_type.into(),
            syntax_function: syntax_function.into(),
            resize_handle_color: if light {
                "#FF4E3A78".into()
            } else {
                "#FF333333".into()
            },
            command_palette_shadow_color: "#66000000".into(),
            workspace_separator_color: "#6EA0A0A0".into(),
            measurement_color: "#FFFFFFFF".into(),
        },
        typography: SettingsTypography {
            editor_font_family: "Consolas".into(),
            editor_fallback_font_family: "Segoe UI Emoji".into(),
            ui_font_family: "Segoe UI".into(),
            workspace_font_size: 12.0,
            editor_font_size: 13.333_333_333_3,
            editor_line_height: 16.0,
            editor_tab_size: 4,
            command_palette_font_size: 14.0,
            welcome_title_font_size: 28.0,
        },
        geometry: SettingsGeometry {
            shell_padding_x: 24.0,
            shell_padding_y: 20.0,
            workspace_sidebar_width: 256.0,
            editor_border_width: 1.0,
            document_tab_min_height: 41.0,
            document_tab_padding_horizontal: 14.4,
            document_tab_padding_vertical: 10.4,
            command_palette_width: 672.0,
            command_palette_padding: 10.4,
            command_palette_max_height: 544.0,
        },
    }
}

pub fn validate(mut settings: AppSettings) -> Result<AppSettings, String> {
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported settings schema version: {}",
            settings.schema_version
        ));
    }

    let color_fields = [
        ("background", &mut settings.colors.background),
        ("foreground", &mut settings.colors.foreground),
        ("selection", &mut settings.colors.selection),
        ("cursor", &mut settings.colors.cursor),
        ("line number", &mut settings.colors.line_number),
        ("gutter background", &mut settings.colors.gutter_background),
        ("gutter separator", &mut settings.colors.gutter_separator),
        ("editor border", &mut settings.colors.editor_border),
        ("diagnostic error", &mut settings.colors.diagnostic_error),
        (
            "diagnostic warning",
            &mut settings.colors.diagnostic_warning,
        ),
        ("diagnostic info", &mut settings.colors.diagnostic_info),
        ("syntax keyword", &mut settings.colors.syntax_keyword),
        ("syntax string", &mut settings.colors.syntax_string),
        ("syntax comment", &mut settings.colors.syntax_comment),
        ("syntax number", &mut settings.colors.syntax_number),
        ("syntax type", &mut settings.colors.syntax_type),
        ("syntax function", &mut settings.colors.syntax_function),
        ("resize handle", &mut settings.colors.resize_handle_color),
        (
            "command palette shadow",
            &mut settings.colors.command_palette_shadow_color,
        ),
        (
            "workspace separator",
            &mut settings.colors.workspace_separator_color,
        ),
        ("measurement", &mut settings.colors.measurement_color),
    ];
    for (name, value) in color_fields {
        normalize_color(name, value)?;
    }

    let font_fields = [
        (
            "editor font family",
            &mut settings.typography.editor_font_family,
        ),
        (
            "editor fallback font family",
            &mut settings.typography.editor_fallback_font_family,
        ),
        ("UI font family", &mut settings.typography.ui_font_family),
    ];
    for (name, value) in font_fields {
        validate_font(name, value)?;
    }

    for (name, value, maximum) in [
        (
            "workspace font size",
            settings.typography.workspace_font_size,
            48.0,
        ),
        (
            "editor font size",
            settings.typography.editor_font_size,
            72.0,
        ),
        (
            "editor line height",
            settings.typography.editor_line_height,
            128.0,
        ),
        (
            "command palette font size",
            settings.typography.command_palette_font_size,
            48.0,
        ),
        (
            "welcome title font size",
            settings.typography.welcome_title_font_size,
            96.0,
        ),
    ] {
        validate_number(name, value, 6.0, maximum)?;
    }
    if !(1..=16).contains(&settings.typography.editor_tab_size) {
        return Err("Editor tab size must be between 1 and 16.".into());
    }

    for (name, value, minimum, maximum) in [
        (
            "horizontal shell padding",
            settings.geometry.shell_padding_x,
            0.0,
            100.0,
        ),
        (
            "vertical shell padding",
            settings.geometry.shell_padding_y,
            0.0,
            100.0,
        ),
        (
            "workspace sidebar width",
            settings.geometry.workspace_sidebar_width,
            160.0,
            800.0,
        ),
        (
            "editor border width",
            settings.geometry.editor_border_width,
            0.0,
            8.0,
        ),
        (
            "document tab minimum height",
            settings.geometry.document_tab_min_height,
            20.0,
            128.0,
        ),
        (
            "document tab horizontal padding",
            settings.geometry.document_tab_padding_horizontal,
            0.0,
            64.0,
        ),
        (
            "document tab vertical padding",
            settings.geometry.document_tab_padding_vertical,
            0.0,
            64.0,
        ),
        (
            "command palette width",
            settings.geometry.command_palette_width,
            240.0,
            1600.0,
        ),
        (
            "command palette padding",
            settings.geometry.command_palette_padding,
            0.0,
            64.0,
        ),
        (
            "command palette maximum height",
            settings.geometry.command_palette_max_height,
            120.0,
            1600.0,
        ),
    ] {
        validate_number(name, value, minimum, maximum)?;
    }
    Ok(settings)
}

pub fn load_settings(config_directory: impl AsRef<Path>) -> Result<AppSettings, String> {
    let path = config_directory.as_ref().join(SETTINGS_FILE_NAME);
    let bytes = match read_bounded_file(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(defaults()),
        Err(error) => {
            return Err(format!(
                "Could not read settings file {}: {error}",
                path.display()
            ));
        }
    };
    parse_settings(&path, &bytes)
}

pub fn save_settings(
    config_directory: impl AsRef<Path>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = validate(settings)?;
    let path = config_directory.as_ref().join(SETTINGS_FILE_NAME);
    write_json_atomically(&path, &settings)?;
    Ok(settings)
}

pub fn load_catalog(config_directory: impl AsRef<Path>) -> Result<SettingsCatalogResponse, String> {
    let config_directory = config_directory.as_ref();
    let settings = load_settings(config_directory)?;
    let (user_themes, warnings) = list_user_themes(config_directory)?;
    Ok(SettingsCatalogResponse {
        settings,
        presets: vec![
            for_preset(ThemePreset::GraphiteDark),
            for_preset(ThemePreset::GraphiteLight),
        ],
        user_themes,
        warnings,
    })
}

pub fn list_user_themes(
    config_directory: impl AsRef<Path>,
) -> Result<(Vec<UserThemeSummary>, Vec<String>), String> {
    let directory = themes_directory(config_directory.as_ref());
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), Vec::new()));
        }
        Err(error) => {
            return Err(format!(
                "Could not read user themes directory {}: {error}",
                directory.display()
            ));
        }
    };
    let mut paths = Vec::new();
    let mut warnings = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Could not inspect user themes directory {}: {error}",
                directory.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Could not inspect user theme {}: {error}",
                entry.path().display()
            )
        })?;
        if file_type.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(THEME_FILE_EXTENSION))
        {
            paths.push(entry.path());
            if paths.len() >= MAX_THEME_FILES_SCANNED {
                record_theme_warning(
                    &mut warnings,
                    "Theme discovery stopped after reaching the file scan limit.".into(),
                );
                break;
            }
        }
    }
    paths.sort();

    let mut themes = Vec::new();
    for path in paths {
        if themes.len() >= MAX_USER_THEMES {
            record_theme_warning(
                &mut warnings,
                "Additional theme files were not loaded because the theme limit was reached."
                    .into(),
            );
            break;
        }
        let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
            record_theme_warning(
                &mut warnings,
                format!("A theme file has an invalid name: {}", path.display()),
            );
            continue;
        };
        if validate_theme_name(name).is_err() || is_builtin_theme_name(name) {
            record_theme_warning(
                &mut warnings,
                format!("Skipped user theme with an invalid or reserved name: {name}"),
            );
            continue;
        }
        match read_theme_file(&path) {
            Ok(settings) => themes.push(UserThemeSummary {
                name: name.to_owned(),
                preset: settings.preset,
            }),
            Err(error) => record_theme_warning(
                &mut warnings,
                format!("Could not load user theme '{}': {error}", name),
            ),
        }
    }
    themes.sort_by_key(|theme| theme.name.to_lowercase());
    Ok((themes, warnings))
}

pub fn load_user_theme(
    config_directory: impl AsRef<Path>,
    name: &str,
) -> Result<AppSettings, String> {
    let name = validate_user_theme_name(name)?;
    let path =
        themes_directory(config_directory.as_ref()).join(format!("{name}.{THEME_FILE_EXTENSION}"));
    read_theme_file(&path)
}

pub fn save_user_theme(
    config_directory: impl AsRef<Path>,
    name: &str,
    settings: AppSettings,
) -> Result<UserThemeSummary, String> {
    let name = validate_user_theme_name(name)?;
    let settings = validate(settings)?;
    let directory = themes_directory(config_directory.as_ref());
    fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create user themes directory {}: {error}",
            directory.display()
        )
    })?;
    if theme_name_exists(&directory, &name)? {
        return Err(format!("A theme named '{name}' already exists."));
    }
    let path = directory.join(format!("{name}.{THEME_FILE_EXTENSION}"));
    write_json_atomically(&path, &settings)?;
    Ok(UserThemeSummary {
        name,
        preset: settings.preset,
    })
}

pub fn validate_user_theme_name(name: &str) -> Result<String, String> {
    let name = validate_theme_name(name)?;
    if is_builtin_theme_name(&name) {
        return Err("Theme name is reserved for a built-in preset.".into());
    }
    Ok(name)
}

fn parse_settings(path: &Path, bytes: &[u8]) -> Result<AppSettings, String> {
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(format!(
            "Settings file {} exceeds the size limit.",
            path.display()
        ));
    }
    let settings = serde_json::from_slice::<AppSettings>(bytes)
        .map_err(|error| format!("Settings file {} is invalid: {error}", path.display()))?;
    validate(settings)
        .map_err(|error| format!("Settings file {} is invalid: {error}", path.display()))
}

fn read_theme_file(path: &Path) -> Result<AppSettings, String> {
    let bytes = read_bounded_file(path)
        .map_err(|error| format!("Could not read theme file {}: {error}", path.display()))?;
    parse_settings(path, &bytes)
}

fn read_bounded_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_SETTINGS_BYTES as u64 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file exceeds the settings size limit",
        ));
    }
    let bytes = fs::read(path)?;
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file exceeds the settings size limit",
        ));
    }
    Ok(bytes)
}

fn write_json_atomically(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Settings path {} has no parent directory.", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Could not create settings directory {}: {error}",
            parent.display()
        )
    })?;
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Could not encode settings: {error}"))?;
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err("Settings data exceeds the size limit.".into());
    }
    let temporary_path = parent.join(format!(
        ".settings.tmp-{}-{}",
        std::process::id(),
        NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(write_error) = fs::write(&temporary_path, bytes) {
        let cleanup_error = fs::remove_file(&temporary_path).err();
        let message = cleanup_error.map_or_else(
            || write_error.to_string(),
            |error| format!("{write_error}; temporary-file cleanup also failed: {error}"),
        );
        return Err(format!(
            "Could not write temporary settings file {}: {message}",
            temporary_path.display()
        ));
    }
    if let Err(rename_error) = fs::rename(&temporary_path, path) {
        let cleanup_error = fs::remove_file(&temporary_path).err();
        let message = cleanup_error.map_or_else(
            || rename_error.to_string(),
            |error| format!("{rename_error}; temporary-file cleanup also failed: {error}"),
        );
        return Err(format!(
            "Could not commit settings file {}: {message}",
            path.display()
        ));
    }
    Ok(())
}

fn themes_directory(config_directory: &Path) -> PathBuf {
    config_directory.join(THEMES_DIRECTORY_NAME)
}

fn validate_theme_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > MAX_THEME_NAME_CHARS
        || name.starts_with('.')
        || name.ends_with('.')
        || name.ends_with(' ')
        || !name
            .chars()
            .all(|character| character.is_alphanumeric() || " _-.".contains(character))
        || is_reserved_windows_name(name)
    {
        return Err(
            "Theme name must be 1-64 letters, numbers, spaces, dots, underscores, or hyphens."
                .into(),
        );
    }
    Ok(name.to_owned())
}

fn is_reserved_windows_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default();
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            upper.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 1
                    && suffix
                        .as_bytes()
                        .first()
                        .is_some_and(|digit| matches!(digit, b'1'..=b'9'))
            })
        })
}

fn is_builtin_theme_name(name: &str) -> bool {
    ["Graphite Dark", "Graphite Light", "Custom"]
        .iter()
        .any(|builtin| builtin.eq_ignore_ascii_case(name))
}

fn theme_name_exists(directory: &Path, name: &str) -> Result<bool, String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("Could not inspect themes directory: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Could not inspect themes directory entry: {error}"))?;
        if entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|existing| existing.eq_ignore_ascii_case(name))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn normalize_color(name: &str, value: &mut String) -> Result<(), String> {
    let color = value.strip_prefix('#').unwrap_or(value.as_str());
    if (color.len() != 6 && color.len() != 8) || !color.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(format!("{name} must be a 6- or 8-digit hexadecimal color."));
    }
    *value = format!("#{}", color.to_ascii_uppercase());
    Ok(())
}

fn validate_font(name: &str, value: &mut String) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 256 || trimmed.chars().any(char::is_control) {
        return Err(format!(
            "{name} must be a non-empty font family of at most 256 characters."
        ));
    }
    *value = trimmed.to_owned();
    Ok(())
}

fn validate_number(name: &str, value: f64, minimum: f64, maximum: f64) -> Result<(), String> {
    if !value.is_finite() || value < minimum || value > maximum {
        return Err(format!("{name} must be between {minimum} and {maximum}."));
    }
    Ok(())
}

fn record_theme_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < MAX_THEME_WARNINGS {
        warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY_ID: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "functor-settings-test-{}-{}",
                std::process::id(),
                NEXT_TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn defaults_round_trip_through_the_versioned_settings_file() {
        let directory = TestDirectory::new();
        save_settings(directory.path(), defaults()).unwrap();
        let mut updated = defaults();
        updated.typography.editor_font_family = "Cascadia Code".into();
        let saved = save_settings(directory.path(), updated).unwrap();
        let loaded = load_settings(directory.path()).unwrap();
        assert_eq!(saved, loaded);
        assert_eq!(loaded.schema_version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(loaded.preset, ThemePreset::GraphiteDark);
    }

    #[test]
    fn built_in_presets_provide_the_graphite_palettes() {
        let dark = for_preset(ThemePreset::GraphiteDark);
        let light = for_preset(ThemePreset::GraphiteLight);
        assert_eq!(dark.colors.background, "#FF17151B");
        assert_eq!(dark.colors.syntax_keyword, "#FFC7A6F7");
        assert_eq!(light.colors.background, "#FFE7E5EA");
        assert_eq!(light.colors.syntax_keyword, "#FF633F9A");
    }

    #[test]
    fn rejects_invalid_colors_fonts_ranges_and_schema_versions() {
        let mut invalid_color = defaults();
        invalid_color.colors.cursor = "nope".into();
        assert!(validate(invalid_color).unwrap_err().contains("cursor"));

        let mut invalid_font = defaults();
        invalid_font.typography.ui_font_family = "  ".into();
        assert!(
            validate(invalid_font)
                .unwrap_err()
                .contains("UI font family")
        );

        let mut invalid_width = defaults();
        invalid_width.geometry.workspace_sidebar_width = f64::INFINITY;
        assert!(
            validate(invalid_width)
                .unwrap_err()
                .contains("workspace sidebar width")
        );

        let mut unsupported = defaults();
        unsupported.schema_version += 1;
        assert!(
            validate(unsupported)
                .unwrap_err()
                .contains("Unsupported settings schema version")
        );
    }

    #[test]
    fn user_themes_round_trip_and_reject_path_like_names() {
        let directory = TestDirectory::new();
        let mut custom = for_preset(ThemePreset::GraphiteLight);
        custom.preset = ThemePreset::Custom;
        custom.typography.editor_font_family = "Cascadia Code".into();
        let saved = save_user_theme(directory.path(), "My Theme", custom.clone()).unwrap();
        let loaded = load_user_theme(directory.path(), &saved.name).unwrap();
        assert_eq!(loaded, custom);
        assert_eq!(list_user_themes(directory.path()).unwrap().0, vec![saved]);
        assert!(save_user_theme(directory.path(), "My Theme", defaults()).is_err());
        assert!(save_user_theme(directory.path(), "../outside", custom).is_err());
    }

    #[test]
    fn invalid_persisted_settings_are_reported_instead_of_replaced_with_defaults() {
        let directory = TestDirectory::new();
        let mut unsupported = defaults();
        unsupported.schema_version = SETTINGS_SCHEMA_VERSION + 1;
        fs::write(
            directory.path().join(SETTINGS_FILE_NAME),
            serde_json::to_vec(&unsupported).unwrap(),
        )
        .unwrap();
        assert!(
            load_settings(directory.path())
                .unwrap_err()
                .contains("Unsupported settings schema version")
        );
    }
}

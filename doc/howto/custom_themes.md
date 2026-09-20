# Custom Themes

Functor includes two built-in themes:

- `Graphite Dark`
- `Graphite Light`

These themes are always available and cannot be overwritten.

## Customize a built-in theme

1. Open the Settings tab.
2. Select `Graphite Dark` or `Graphite Light`.
3. Adjust the colors, fonts, sizes, or other theme values.
4. Enter a new name in the `Theme name` field, such as `My Graphite Dark`.
5. Select `Export theme`.

Functor saves the theme as:

```text
%APPDATA%\Functor\themes\My Graphite Dark.functortheme
```

The original built-in theme remains unchanged.

## Customize an existing user theme

1. Select the user theme from the theme dropdown.
2. Modify its settings.
3. Enter a different name in the `Theme name` field.
4. Select `Export theme`.

The original theme file remains unchanged. The edited values are saved as a new `.functortheme` file.

## Apply and save settings

- `Apply` applies the current settings for the running application.
- `Save` applies the settings and saves them as the active application settings in `settings.json`.
- `Export theme` creates a shareable `.functortheme` file in the themes folder.

`Save` does not modify the selected built-in or user theme file. Use `Export theme` to create a theme file.

## Theme names

Theme names must:

- Contain at least one character.
- Be valid Windows file names.
- Not match `Graphite Dark` or `Graphite Light`.
- Not match an existing user theme, ignoring letter case.

For example, if `Ocean.functortheme` already exists, exporting as `ocean` is rejected. Choose another name instead.

## Sharing themes

Share the generated `.functortheme` file with another user. They can copy it into:

```text
%APPDATA%\Functor\themes
```

The theme will appear in the Settings dropdown the next time the Settings tab is opened. Invalid theme files and files with other extensions are ignored.

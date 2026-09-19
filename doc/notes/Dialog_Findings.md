# Dialog Findings

## Current Dialog Surface

Functor currently has one application-created modal dialog: the unsaved-changes confirmation in `ShellHostView`.

It previously created a raw Avalonia `Window`, applied theme resources, and supplied a content stack. That gave the dialog themed controls but left its window chrome inconsistent with the custom `MainWindow` chrome.

## Shared Dialog Path

`ThemedDialogWindow.create` is now the standard construction path for application dialogs. It provides:

- border-only window decorations with the client area extended into the chrome;
- a 32-pixel themed title bar using the active panel, border, and primary-text resources;
- a themed bordered surface for dialog content;
- title-bar dragging;
- active `AppSettings` application before dialog content is created.

The helper returns the `Window` and a `setContent` function so each dialog owns only its content and actions.

## Future Dialog Rule

New application dialogs should use `ThemedDialogWindow.create` instead of constructing `Window` directly. This keeps theme application, light/dark variant selection, border treatment, title-bar behavior, and content framing in one place.

If a future dialog needs resizable behavior, custom chrome controls, or a different title-bar height, extend the helper with an explicit option rather than duplicating the window setup at the call site.

The helper assigns the active resource values when the dialog is created and subscribes to the window's public `ResourcesChanged` event. This refreshes the chrome when `ThemeManager.apply` replaces application resources, including for an already-open dialog. Avalonia's `DynamicResourceExtension` remains a XAML markup extension whose binding application is internal, so the resource-host event is the supported code-behind approach here.

## Scope Boundary

Native operating-system dialogs and Avalonia platform dialogs are not controlled by this helper. They should retain their platform behavior unless the application replaces them with an application-owned dialog.

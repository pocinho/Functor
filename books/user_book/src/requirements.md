# Requirements and Troubleshooting

The Tauri desktop application requires the operating-system WebView runtime.
On Windows, install the Microsoft Edge WebView2 Runtime. macOS uses the
system WebKit framework; Linux requires WebKitGTK 4.1 and its distribution's
native packages.

The transition build does not include a general-purpose installer. The
developer book lists the complete platform and development prerequisites.

If the editor does not start, first verify the WebView runtime and consult the
developer book for platform-specific launch and validation status.

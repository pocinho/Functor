namespace Functor.Workspace

/// Settings owned by a workspace and reset when the workspace changes.
type WorkspaceSettings =
    { IndentSize: int
      InsertFinalNewline: bool
      TrimTrailingWhitespace: bool
      DefaultLanguage: string option }

module WorkspaceSettings =
    let defaults =
        { IndentSize = 4
          InsertFinalNewline = false
          TrimTrailingWhitespace = false
          DefaultLanguage = None }

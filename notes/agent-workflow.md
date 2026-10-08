# Agent Workflow Proposal

## Status

This document records a future design direction. The current Agent panel is a
placeholder; conversation, context sharing, and agent integration are not
implemented.

## Workspace-aware conversation

The Agent panel should work like a conventional chat panel: conversation
history and agent responses occupy the upper, scrollable area, while a user
input composer stays at the bottom. It is a workspace-level panel, independent
of the active document.

The workspace surface is the area shown inside the red outline in the proposed
layout: the workspace navigation/explorer and the main editor. The Agent panel
sits beside that surface and can help with the file or selection the user is
currently working on. Its context should follow the user's focus without
making every file in the workspace implicit input.

## Context shared with an agent

Prefer structured, typed application context over a screenshot for normal
coding assistance. A turn may include:

- workspace identity and relevant explorer selection or loaded-tree metadata;
- the active document's stable identity, workspace-relative path, revision,
  and current text, subject to context-size limits;
- the active selection and cursor position;
- metadata about other open tabs, without their contents by default.

The panel should make the proposed context visible before sending a prompt,
for example with context indicators for the workspace, file, and selection.
Users should be able to remove or add context. Broad workspace context should
be gathered through explicit, scoped reads or search operations rather than by
silently sending all project files.

An image of the workspace surface may be useful for visual or layout questions.
If supported, capture should be user-initiated and limited to the relevant
application region; it should be optional rather than the default source of
code context.

## State ownership and permissions

Build context from Rust-owned document/workspace state and typed bridge
projections, not by scraping Monaco models or treating the DOM as canonical.
Keep Tauri, WebView, and unrestricted filesystem access out of agent-facing
domain logic.

Agent tools should use narrow, permissioned operations with stable document
identities and revision checks. Read and search scope must be explicit. Proposed
edits should be reviewable and confirmed by the user, then applied through the
normal validated document-update path; an agent must not mutate a Monaco model
or project file behind Rust's state. Surface failures and cancellation
explicitly. Treat workspace content as untrusted input, not as authority to
expand an agent's permissions.

## Decisions still open

- Agent providers, plugin integration, and transport.
- Conversation persistence, retention, and workspace association.
- Context size limits and how much of the active document is sent by default.
- Which reads or edits require per-action approval versus a broader user grant.
- Whether and how to support visual captures of the workspace surface.
- Streaming, cancellation, and recovery of interrupted responses.

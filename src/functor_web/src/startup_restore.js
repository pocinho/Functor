export async function restoreStartupDocuments(bridgeInvoke, renderSnapshot, describeError) {
  let workspace = null;
  const warnings = [];

  try {
    workspace = await bridgeInvoke("restore_workspace_documents");
    renderSnapshot(workspace.snapshot);
    warnings.push(
      ...workspace.warnings.map((warning) => `Workspace restore warning: ${warning}`),
    );
  } catch (error) {
    warnings.push(`Workspace tabs could not be restored: ${describeError(error)}`);
  }

  try {
    const recovered = await bridgeInvoke("restore_recovery_snapshots");
    renderSnapshot(recovered.snapshot, true);
    warnings.push(...recovered.warnings.map((warning) => `Recovery warning: ${warning}`));
  } catch (error) {
    warnings.push(`Unsaved work could not be restored: ${describeError(error)}`);
  }

  return { workspace, warnings };
}

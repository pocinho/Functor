import assert from "node:assert/strict";
import { test } from "node:test";
import { restoreStartupDocuments } from "../src/startup_restore.js";

test("restores workspace tabs before unsaved snapshots", async () => {
  const commands = [];
  const rendered = [];
  const result = await restoreStartupDocuments(
    async (command) => {
      commands.push(command);
      return {
        snapshot: { name: command },
        warnings: command === "restore_workspace_documents" ? ["missing file"] : [],
      };
    },
    (snapshot, focus) => rendered.push({ snapshot, focus: Boolean(focus) }),
    String,
  );

  assert.deepEqual(commands, ["restore_workspace_documents", "restore_recovery_snapshots"]);
  assert.deepEqual(rendered, [
    { snapshot: { name: "restore_workspace_documents" }, focus: false },
    { snapshot: { name: "restore_recovery_snapshots" }, focus: true },
  ]);
  assert.deepEqual(result.warnings, ["Workspace restore warning: missing file"]);
  assert.equal(result.workspace.snapshot.name, "restore_workspace_documents");
});

test("still attempts recovery when workspace tabs cannot be restored", async () => {
  const commands = [];
  const result = await restoreStartupDocuments(
    async (command) => {
      commands.push(command);
      if (command === "restore_workspace_documents") {
        throw new Error("workspace unavailable");
      }
      return { snapshot: { documents: [] }, warnings: ["source changed"] };
    },
    () => {},
    (error) => error.message,
  );

  assert.deepEqual(commands, ["restore_workspace_documents", "restore_recovery_snapshots"]);
  assert.equal(result.workspace, null);
  assert.deepEqual(result.warnings, [
    "Workspace tabs could not be restored: workspace unavailable",
    "Recovery warning: source changed",
  ]);
});

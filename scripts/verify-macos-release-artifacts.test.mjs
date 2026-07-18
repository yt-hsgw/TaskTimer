import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import {
  resolveMacOSArtifacts,
  verifyMacOSArtifacts,
} from "./verify-macos-release-artifacts.mjs";

function withArtifacts(run) {
  const rootDir = mkdtempSync(join(tmpdir(), "tasktimer-macos-release-"));
  const app = join(rootDir, "TaskTimer.app");
  const dmg = join(rootDir, "TaskTimer.dmg");
  mkdirSync(app);
  writeFileSync(dmg, "fixture");

  try {
    run({ rootDir, app, dmg });
  } finally {
    rmSync(rootDir, { recursive: true, force: true });
  }
}

test("Tauri Action出力から.appと.dmgを解決する", () => {
  withArtifacts(({ rootDir, app, dmg }) => {
    assert.deepEqual(resolveMacOSArtifacts(JSON.stringify([app, dmg]), rootDir), {
      app: realpathSync(app),
      dmg: realpathSync(dmg),
    });
  });
});

test("相対pathをリポジトリルート基準で解決する", () => {
  withArtifacts(({ rootDir, app, dmg }) => {
    assert.deepEqual(
      resolveMacOSArtifacts(JSON.stringify(["TaskTimer.app", "TaskTimer.dmg"]), rootDir),
      { app: realpathSync(app), dmg: realpathSync(dmg) },
    );
  });
});

test("壊れたartifactPaths JSONを拒否する", () => {
  withArtifacts(({ rootDir }) => {
    assert.throws(
      () => resolveMacOSArtifacts("not-json", rootDir),
      /不正なJSON/,
    );
  });
});

test(".appまたは.dmgが不足した出力を拒否する", () => {
  withArtifacts(({ rootDir, app }) => {
    assert.throws(
      () => resolveMacOSArtifacts(JSON.stringify([app]), rootDir),
      /1件ずつ必要/,
    );
  });
});

test("リポジトリ外の成果物を拒否する", () => {
  const rootDir = mkdtempSync(join(tmpdir(), "tasktimer-root-"));
  const outsideDir = mkdtempSync(join(tmpdir(), "tasktimer-outside-"));
  const app = join(outsideDir, "TaskTimer.app");
  const dmg = join(outsideDir, "TaskTimer.dmg");
  mkdirSync(app);
  writeFileSync(dmg, "fixture");

  try {
    assert.throws(
      () => resolveMacOSArtifacts(JSON.stringify([app, dmg]), rootDir),
      /リポジトリ外/,
    );
  } finally {
    rmSync(rootDir, { recursive: true, force: true });
    rmSync(outsideDir, { recursive: true, force: true });
  }
});

test("Apple標準コマンドで署名・Gatekeeper・公証チケットを検証する", () => {
  const calls = [];
  const signatureDetails = [
    "Authority=Developer ID Application: Example (TEAMID)",
    "Timestamp=Jul 19, 2026 at 12:00:00",
    "flags=0x10000(runtime)",
  ].join("\n");
  const execute = (command, args) => {
    calls.push([command, args]);
    return command === "codesign" && args.includes("--display")
      ? { status: 0, stdout: "", stderr: signatureDetails }
      : { status: 0, stdout: "", stderr: "" };
  };

  verifyMacOSArtifacts(
    { app: "/repo/TaskTimer.app", dmg: "/repo/TaskTimer.dmg" },
    { rootDir: "/repo", platform: "darwin", execute },
  );

  assert.equal(calls.length, 6);
  assert.deepEqual(calls[2], [
    "spctl",
    ["--assess", "--type", "execute", "--verbose=3", "/repo/TaskTimer.app"],
  ]);
  assert.deepEqual(calls[5], [
    "codesign",
    ["--display", "--verbose=4", "/repo/TaskTimer.dmg"],
  ]);
});

test("ad-hoc署名を拒否する", () => {
  const execute = (command, args) =>
    command === "codesign" && args.includes("--display")
      ? { status: 0, stdout: "", stderr: "Signature=adhoc\nflags=0x10000(runtime)" }
      : { status: 0, stdout: "", stderr: "" };

  assert.throws(
    () =>
      verifyMacOSArtifacts(
        { app: "/repo/TaskTimer.app", dmg: "/repo/TaskTimer.dmg" },
        { rootDir: "/repo", platform: "darwin", execute },
      ),
    /Developer ID Application署名ではありません/,
  );
});

test("安全なタイムスタンプがないDeveloper ID署名を拒否する", () => {
  const execute = (command, args) =>
    command === "codesign" && args.includes("--display")
      ? {
          status: 0,
          stdout: "",
          stderr: "Authority=Developer ID Application: Example (TEAMID)\nflags=0x10000(runtime)",
        }
      : { status: 0, stdout: "", stderr: "" };

  assert.throws(
    () =>
      verifyMacOSArtifacts(
        { app: "/repo/TaskTimer.app", dmg: "/repo/TaskTimer.dmg" },
        { rootDir: "/repo", platform: "darwin", execute },
      ),
    /安全なタイムスタンプがありません/,
  );
});

test("Hardened Runtimeが無効な.appを拒否する", () => {
  const execute = (command, args) =>
    command === "codesign" && args.includes("--display")
      ? {
          status: 0,
          stdout: "",
          stderr: [
            "Authority=Developer ID Application: Example (TEAMID)",
            "Timestamp=Jul 19, 2026 at 12:00:00",
            "flags=0x0(none)",
          ].join("\n"),
        }
      : { status: 0, stdout: "", stderr: "" };

  assert.throws(
    () =>
      verifyMacOSArtifacts(
        { app: "/repo/TaskTimer.app", dmg: "/repo/TaskTimer.dmg" },
        { rootDir: "/repo", platform: "darwin", execute },
      ),
    /Hardened Runtimeが有効ではありません/,
  );
});

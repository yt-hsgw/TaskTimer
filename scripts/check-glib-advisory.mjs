#!/usr/bin/env node
import { copyFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifestPath = join(rootDir, "src-tauri", "Cargo.toml");
const lockPath = join(rootDir, "src-tauri", "Cargo.lock");
const releasePolicyScript = join(rootDir, "scripts", "check-release-platform-policy.mjs");
const lockBackupPath = join(tmpdir(), `tasktimer-Cargo.lock.${process.pid}.bak`);
const targetPolicies = [
  { target: "x86_64-pc-windows-msvc", shouldContainGlib: false },
  { target: "aarch64-apple-darwin", shouldContainGlib: false },
  { target: "x86_64-apple-darwin", shouldContainGlib: false },
  { target: "x86_64-unknown-linux-gnu", shouldContainGlib: true },
];

function runCargo(args, options = {}) {
  const result = spawnSync("cargo", args, {
    cwd: rootDir,
    encoding: "utf8",
    maxBuffer: 20 * 1024 * 1024,
    ...options,
  });

  if (options.stdio === "inherit") {
    return result;
  }

  return {
    ...result,
    outputText: `${result.stdout ?? ""}${result.stderr ?? ""}`,
  };
}

function parseVersion(version) {
  return version
    .split(".")
    .map((part) => Number.parseInt(part.replace(/\D.*/, ""), 10) || 0);
}

function compareVersions(left, right) {
  const leftParts = parseVersion(left);
  const rightParts = parseVersion(right);

  for (let index = 0; index < 3; index += 1) {
    if (leftParts[index] > rightParts[index]) {
      return 1;
    }
    if (leftParts[index] < rightParts[index]) {
      return -1;
    }
  }

  return 0;
}

function isVulnerableGlib(version) {
  return compareVersions(version, "0.15.0") >= 0 && compareVersions(version, "0.20.0") < 0;
}

function restoreLockfile() {
  if (existsSync(lockBackupPath)) {
    copyFileSync(lockBackupPath, lockPath);
    rmSync(lockBackupPath, { force: true });
  }
}

if (!existsSync(lockPath)) {
  console.error(`::error title=Cargo.lock not found::${lockPath} が見つかりません`);
  process.exit(1);
}

function main() {
  copyFileSync(lockPath, lockBackupPath);

  try {
    const releasePolicyResult = spawnSync(process.execPath, [releasePolicyScript], {
      cwd: rootDir,
      stdio: "inherit",
    });
    if (releasePolicyResult.status !== 0) {
      return releasePolicyResult.status ?? 1;
    }

    console.log("Cargo.lockを一時更新し、Tauri/GTK系の最新resolverでglib advisoryを再評価します。");
    const updateResult = runCargo(["update", "--manifest-path", manifestPath], { stdio: "inherit" });
    if (updateResult.status !== 0) {
      console.error("::error title=cargo update failed::依存関係の一時更新に失敗しました");
      return updateResult.status ?? 1;
    }

    const metadataResult = runCargo([
      "metadata",
      "--manifest-path",
      manifestPath,
      "--format-version",
      "1",
    ]);

    if (metadataResult.status !== 0) {
      console.error("::error title=cargo metadata failed::Cargo metadataを取得できませんでした");
      console.error(metadataResult.error?.message ?? metadataResult.stderr);
      return metadataResult.status ?? 1;
    }

    const metadata = JSON.parse(metadataResult.stdout);
    const glibPackages = metadata.packages.filter((pkg) => pkg.name === "glib");
    const vulnerableGlibPackages = glibPackages.filter((pkg) => isVulnerableGlib(pkg.version));

    for (const policy of targetPolicies) {
      const treeResult = runCargo([
        "tree",
        "--manifest-path",
        manifestPath,
        "--target",
        policy.target,
        "-i",
        "glib",
      ]);
      if (treeResult.status !== 0) {
        console.error(
          `::error title=cargo tree failed::${policy.target}の依存グラフを確認できませんでした`,
        );
        console.error(treeResult.outputText);
        return treeResult.status ?? 1;
      }
      const containsGlib = /^glib v/m.test(treeResult.stdout ?? "");
      if (containsGlib !== policy.shouldContainGlib) {
        console.error(
          `::error title=unexpected glib target boundary::${policy.target}のglib依存状態がポリシーと一致しません`,
        );
        return 1;
      }
      console.log(
        `${policy.target}: glib ${containsGlib ? "あり（Linux限定・未解消）" : "なし"}`,
      );
    }

    if (glibPackages.length === 0) {
      console.error(
        "::error title=glib dependency removed::glibが依存グラフから消えました。Dependabot alert #1を確認してください",
      );
      return 1;
    }

    if (vulnerableGlibPackages.length === 0) {
      console.error(
        "::error title=glib advisory can be remediated::glib 0.20.0以上へ更新可能です。依存更新PRを作成し、Issue #22を解消してください",
      );
      for (const pkg of glibPackages) {
        console.error(`検出したglib: ${pkg.version}`);
      }
      return 1;
    }

    const preciseResult = runCargo([
      "update",
      "--manifest-path",
      manifestPath,
      "-p",
      "glib",
      "--precise",
      "0.20.0",
    ]);

    if (preciseResult.status === 0) {
      console.error(
        "::error title=glib 0.20.0 resolved::glib 0.20.0指定が成功しました。Cargo.lock更新PRを作成し、Issue #22を解消してください",
      );
      return 1;
    }

    const knownBlocked =
      preciseResult.outputText.includes('glib = "^0.18"') ||
      preciseResult.outputText.includes("gtk v0.18.2");

    if (!knownBlocked) {
      console.error("::error title=unexpected glib advisory state::glib advisoryのブロック理由が想定と異なります");
      console.error(preciseResult.outputText);
      return 1;
    }

    console.log(
      "::notice title=glib advisory still blocked::gtk 0.18系の制約によりglib 0.20.0以上へ更新できません",
    );
    for (const pkg of vulnerableGlibPackages) {
      console.log(`検出した脆弱対象glib: ${pkg.version}`);
    }
    console.log("Linuxでは未解消です。Windows/macOS限定配布と週次監視を継続してください。");
    return 0;
  } finally {
    restoreLockfile();
  }
}

process.exit(main());

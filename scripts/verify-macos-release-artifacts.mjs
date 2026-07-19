#!/usr/bin/env node
import { existsSync, realpathSync, statSync } from "node:fs";
import { dirname, extname, isAbsolute, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const defaultRootDir = resolve(dirname(scriptPath), "..");

function isWithinRoot(rootDir, path) {
  const relativePath = relative(rootDir, path);
  return relativePath !== "" && !relativePath.startsWith("..") && !isAbsolute(relativePath);
}

export function resolveMacOSArtifacts(rawArtifactPaths, rootDir = defaultRootDir) {
  if (!rawArtifactPaths) {
    throw new Error("Tauri ActionのartifactPaths出力がありません");
  }

  let artifactPaths;
  try {
    artifactPaths = JSON.parse(rawArtifactPaths);
  } catch {
    throw new Error("Tauri ActionのartifactPaths出力が不正なJSONです");
  }

  if (!Array.isArray(artifactPaths) || artifactPaths.some((path) => typeof path !== "string")) {
    throw new Error("Tauri ActionのartifactPaths出力は文字列配列である必要があります");
  }

  const resolvedRoot = realpathSync(rootDir);
  const candidates = artifactPaths
    .filter((path) => extname(path) === ".app" || extname(path) === ".dmg")
    .map((path) => {
      const candidatePath = isAbsolute(path) ? path : resolve(resolvedRoot, path);
      if (!existsSync(candidatePath)) {
        throw new Error(`macOS成果物が見つかりません: ${extname(path) || "不明"}`);
      }

      const resolvedPath = realpathSync(candidatePath);
      if (!isWithinRoot(resolvedRoot, resolvedPath)) {
        throw new Error("リポジトリ外のmacOS成果物は検証できません");
      }
      return resolvedPath;
    });

  const apps = candidates.filter((path) => extname(path) === ".app");
  const dmgs = candidates.filter((path) => extname(path) === ".dmg");

  if (apps.length !== 1 || dmgs.length !== 1) {
    throw new Error("検証対象はアーキテクチャごとに.appと.dmgが1件ずつ必要です");
  }

  if (!statSync(apps[0]).isDirectory() || !statSync(dmgs[0]).isFile()) {
    throw new Error(".appはディレクトリ、.dmgはファイルである必要があります");
  }

  return { app: apps[0], dmg: dmgs[0] };
}

function run(command, args, rootDir) {
  return spawnSync(command, args, {
    cwd: rootDir,
    encoding: "utf8",
  });
}

function assertSucceeded(result, label) {
  if (result.error || result.status !== 0) {
    throw new Error(`${label}に失敗しました`);
  }
}

function assertDeveloperIdSignature(result, label) {
  assertSucceeded(result, label);
  const details = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;

  if (!/^Authority=Developer ID Application:/m.test(details)) {
    throw new Error(`${label}がDeveloper ID Application署名ではありません`);
  }
  if (!/^Timestamp=/m.test(details)) {
    throw new Error(`${label}に安全なタイムスタンプがありません`);
  }
}

export function verifyMacOSArtifacts(
  artifacts,
  { rootDir = defaultRootDir, platform = process.platform, execute = run } = {},
) {
  if (platform !== "darwin") {
    throw new Error("macOS成果物検証はmacOS上でのみ実行できます");
  }

  const invoke = (command, args) => execute(command, args, rootDir);

  assertSucceeded(
    invoke("codesign", ["--verify", "--deep", "--strict", "--verbose=3", artifacts.app]),
    ".appのコード署名検証",
  );
  const appSignature = invoke("codesign", ["--display", "--verbose=4", artifacts.app]);
  assertDeveloperIdSignature(appSignature, ".appの署名情報検証");
  const appSignatureDetails = `${appSignature.stdout ?? ""}\n${appSignature.stderr ?? ""}`;
  if (!/^flags=.*\bruntime\b/m.test(appSignatureDetails)) {
    throw new Error(".appでHardened Runtimeが有効ではありません");
  }

  assertSucceeded(
    invoke("spctl", ["--assess", "--type", "execute", "--verbose=3", artifacts.app]),
    ".appのGatekeeper評価",
  );
  assertSucceeded(
    invoke("xcrun", ["stapler", "validate", artifacts.app]),
    ".appの公証チケット検証",
  );

  assertSucceeded(
    invoke("codesign", ["--verify", "--strict", "--verbose=3", artifacts.dmg]),
    ".dmgのコード署名検証",
  );
  assertDeveloperIdSignature(
    invoke("codesign", ["--display", "--verbose=4", artifacts.dmg]),
    ".dmgの署名情報検証",
  );
}

function main() {
  try {
    const artifacts = resolveMacOSArtifacts(process.env.TAURI_ARTIFACT_PATHS);
    verifyMacOSArtifacts(artifacts);
    console.log("macOSの.appと.dmgの署名・公証成果物検証に成功しました。");
  } catch (error) {
    const message = error instanceof Error ? error.message : "不明なエラー";
    console.error(`::error title=macOS release artifact verification failed::${message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  main();
}

#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packageJsonPath = join(rootDir, "package.json");
const cargoTomlPath = join(rootDir, "src-tauri", "Cargo.toml");
const tauriConfigPath = join(rootDir, "src-tauri", "tauri.conf.json");

function fail(message) {
  console.error(`::error title=release target check failed::${message}`);
  process.exit(1);
}

function runGit(args) {
  const result = spawnSync("git", args, {
    cwd: rootDir,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    fail(result.stderr.trim() || `git ${args.join(" ")} に失敗しました`);
  }

  return result.stdout.trim();
}

function readCargoVersion() {
  const cargoToml = readFileSync(cargoTomlPath, "utf8");
  const versionMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);

  if (!versionMatch) {
    fail("src-tauri/Cargo.toml のversionを読み取れませんでした");
  }

  return versionMatch[1];
}

const packageVersion = JSON.parse(readFileSync(packageJsonPath, "utf8")).version;
const tauriVersion = JSON.parse(readFileSync(tauriConfigPath, "utf8")).version;
const cargoVersion = readCargoVersion();

if (packageVersion !== tauriVersion || packageVersion !== cargoVersion) {
  fail(
    `version不整合: package.json=${packageVersion}, tauri.conf.json=${tauriVersion}, Cargo.toml=${cargoVersion}`,
  );
}

const requestedVersion = process.argv[2] ?? packageVersion;
const targetRef = process.argv[3] ?? "HEAD";
const releaseVersion = requestedVersion.startsWith("app-v")
  ? requestedVersion.slice("app-v".length)
  : requestedVersion;
const tagName = `app-v${releaseVersion}`;

if (releaseVersion !== packageVersion) {
  fail(`指定version ${releaseVersion} がプロジェクトversion ${packageVersion} と一致しません`);
}

const targetCommit = runGit(["rev-parse", `${targetRef}^{commit}`]);
const tagCommit = runGit(["rev-parse", `${tagName}^{commit}`]);

if (targetCommit !== tagCommit) {
  console.error(`Release tag ${tagName} は意図したリリースコミットを指していません。`);
  console.error(`tag commit:    ${tagCommit}`);
  console.error(`target commit: ${targetCommit}`);
  console.error("");
  console.error("Draft Releaseを公開せず、必要ならDraft Releaseとtagを作り直してください。");
  console.error("");
  console.error("確認用コマンド:");
  console.error(`  git fetch origin main --tags`);
  console.error(`  npm run check:release-target -- ${releaseVersion} origin/main`);
  fail(`${tagName} と ${targetRef} のcommitが一致しません`);
}

console.log(`Release tag ${tagName} は ${targetRef} (${targetCommit}) を指しています。`);

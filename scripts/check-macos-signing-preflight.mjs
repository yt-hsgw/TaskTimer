#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tauriConfigPath = join(rootDir, "src-tauri", "tauri.conf.json");
const entitlementsPath = join(rootDir, "src-tauri", "Entitlements.plist");

const requiredSecrets = [
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
];

const disallowedEntitlements = [
  "com.apple.security.network.client",
  "com.apple.security.network.server",
  "com.apple.security.files.user-selected.read-only",
  "com.apple.security.files.user-selected.read-write",
  "com.apple.security.personal-information.location",
  "com.apple.security.device.camera",
  "com.apple.security.device.microphone",
];

function fail(message) {
  console.error(`::error title=macOS signing preflight failed::${message}`);
  process.exitCode = 1;
}

function notice(message) {
  console.log(`::notice title=macOS signing preflight::${message}`);
}

function warn(message) {
  console.warn(`::warning title=macOS signing preflight warning::${message}`);
}

function run(command, args) {
  return spawnSync(command, args, {
    cwd: rootDir,
    encoding: "utf8",
  });
}

function checkTauriConfig() {
  if (!existsSync(tauriConfigPath)) {
    fail("src-tauri/tauri.conf.json が見つかりません");
    return;
  }

  const config = JSON.parse(readFileSync(tauriConfigPath, "utf8"));
  const bundle = config.bundle ?? {};
  const targets = Array.isArray(bundle.targets) ? bundle.targets : [];
  const macOS = bundle.macOS ?? {};

  if (bundle.active !== true) {
    fail("Tauri bundle.active が true ではありません");
  }

  if (!targets.includes("dmg")) {
    fail("Tauri bundle.targets に dmg が含まれていません");
  }

  if (macOS.hardenedRuntime !== true) {
    fail("Tauri bundle.macOS.hardenedRuntime が true ではありません");
  }

  if (macOS.entitlements !== "./Entitlements.plist") {
    fail("Tauri bundle.macOS.entitlements が ./Entitlements.plist ではありません");
  }

  if (bundle.createUpdaterArtifacts !== false) {
    fail("MVPでは createUpdaterArtifacts を false にしてください");
  }
}

function checkEntitlements() {
  if (!existsSync(entitlementsPath)) {
    fail("src-tauri/Entitlements.plist が見つかりません");
    return;
  }

  const entitlements = readFileSync(entitlementsPath, "utf8");

  for (const key of disallowedEntitlements) {
    if (entitlements.includes(key)) {
      fail(`Entitlements.plist に不要な権限 ${key} が含まれています`);
    }
  }
}

function checkGitHubSecrets() {
  const repoResult = run("gh", ["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"]);
  if (repoResult.status !== 0) {
    fail("GitHub CLIでリポジトリを特定できません。gh auth status とremote設定を確認してください");
    return;
  }

  const repo = repoResult.stdout.trim();
  const secretsResult = run("gh", [
    "secret",
    "list",
    "--repo",
    repo,
    "--app",
    "actions",
    "--json",
    "name,updatedAt",
  ]);

  if (secretsResult.status !== 0) {
    fail("GitHub Actions Secrets一覧を取得できません。gh auth status と権限を確認してください");
    return;
  }

  const secrets = JSON.parse(secretsResult.stdout);
  const registeredSecretNames = new Set(secrets.map((secret) => secret.name));
  const missingSecrets = requiredSecrets.filter((name) => !registeredSecretNames.has(name));

  if (missingSecrets.length > 0) {
    fail(`GitHub Actions Secretsが不足しています: ${missingSecrets.join(", ")}`);
  } else {
    notice("macOS署名・公証に必要なGitHub Actions Secrets名は登録済みです");
  }
}

function checkLocalMacTools() {
  if (process.platform !== "darwin") {
    warn("macOS以外では codesign / notarytool / stapler のローカル確認をスキップします");
    return;
  }

  const commands = [
    ["xcrun", ["--find", "codesign"]],
    ["security", ["find-identity", "-v", "-p", "codesigning"]],
    ["xcrun", ["--find", "notarytool"]],
    ["xcrun", ["--find", "stapler"]],
  ];

  for (const [command, args] of commands) {
    const result = run(command, args);
    if (result.status !== 0) {
      fail(`${command} ${args.join(" ")} を実行できません`);
    }
  }
}

checkTauriConfig();
checkEntitlements();
checkGitHubSecrets();
checkLocalMacTools();

if (process.exitCode && process.exitCode !== 0) {
  console.error("macOS署名・公証のpreflightに失敗しました。Draft Releaseを公開しないでください。");
} else {
  console.log("macOS署名・公証のpreflightに成功しました。Release workflow実行後、実機でGatekeeper確認を続けてください。");
}

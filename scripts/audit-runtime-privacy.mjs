#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const findings = [];

const runtimeSourceTargets = [
  { dir: "src", extensions: new Set([".css", ".ts", ".tsx"]) },
  { dir: "src-tauri/src", extensions: new Set([".rs"]) },
];

const runtimeSourceExclusions = new Set([
  // 開発・検証用CLI。Tauriアプリ本体には組み込まれず、合成データ生成結果だけを標準出力へ表示する。
  "src-tauri/src/bin/seed_large_dataset.rs",
  // 開発・検証用CLI。Tauriアプリ本体には組み込まれず、合成DBのRead Model計測結果だけを標準出力へ表示する。
  "src-tauri/src/bin/measure_large_dataset.rs",
]);

const jsForbiddenPatterns = [
  {
    id: "runtime-network-api",
    pattern: /\b(fetch\s*\(|XMLHttpRequest\b|WebSocket\b|EventSource\b|navigator\.sendBeacon\b)/,
    message: "実行時コードで外部通信APIを使っています",
  },
  {
    id: "runtime-console-log",
    pattern: /\bconsole\.(log|debug|info|warn|error)\s*\(/,
    message: "実行時コードでconsoleログを出しています",
  },
  {
    id: "runtime-remote-url",
    pattern: /https?:\/\//,
    message: "実行時コードにリモートURLが含まれています",
  },
  {
    id: "runtime-css-import",
    pattern: /@import\b/i,
    message: "CSS importはリモートフォント/外部CSS混入の入口になります",
  },
];

const rustForbiddenPatterns = [
  {
    id: "runtime-rust-network",
    pattern: /\b(reqwest|ureq|hyper|TcpListener|TcpStream|UdpSocket)\b/,
    message: "Rust実行時コードでネットワーク系APIを参照しています",
  },
  {
    id: "runtime-rust-log",
    pattern: /\b(println!|eprintln!|dbg!|tracing::|log::)/,
    message: "Rust実行時コードでログ出力系APIを参照しています",
  },
];

const forbiddenNpmRuntimeDependencies = [
  "@sentry/browser",
  "@sentry/react",
  "@tauri-apps/plugin-http",
  "@tauri-apps/plugin-updater",
  "axios",
  "posthog-js",
  "react-ga",
  "react-ga4",
  "undici",
];

const forbiddenCargoDependencies = [
  "hyper",
  "reqwest",
  "tauri-plugin-http",
  "tauri-plugin-updater",
  "tokio-tungstenite",
  "tungstenite",
  "ureq",
];

const forbiddenCapabilityFragments = [
  "http",
  "opener",
  "shell",
  "updater",
  "websocket",
];

let checkedFiles = 0;

for (const target of runtimeSourceTargets) {
  const absoluteDir = join(repoRoot, target.dir);
  for (const filePath of walkFiles(absoluteDir)) {
    if (!target.extensions.has(extname(filePath))) {
      continue;
    }
    if (runtimeSourceExclusions.has(relative(repoRoot, filePath))) {
      continue;
    }
    checkedFiles += 1;
    checkRuntimeSource(filePath);
  }
}

checkPackageJson();
checkCargoToml();
checkTauriConfig();
checkCapabilities();
checkMacOSEntitlements();

if (findings.length > 0) {
  console.error("実行時プライバシー監査に失敗しました。");
  for (const finding of findings) {
    console.error(`- ${finding}`);
  }
  process.exit(1);
}

console.log(`実行時プライバシー監査: OK (${checkedFiles} files checked)`);
console.log("外部通信API、実行時ログ出力、リモートアセット、更新機能権限は検出されませんでした。");

function checkRuntimeSource(filePath) {
  const relativePath = relative(repoRoot, filePath);
  const content = readFileSync(filePath, "utf8");
  const patterns = filePath.endsWith(".rs") ? rustForbiddenPatterns : jsForbiddenPatterns;
  const lines = content.split(/\r?\n/);

  for (const [index, line] of lines.entries()) {
    for (const rule of patterns) {
      if (rule.pattern.test(line)) {
        addFinding(`${relativePath}:${index + 1} [${rule.id}] ${rule.message}`);
      }
    }
  }
}

function checkPackageJson() {
  const packageJson = JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
  const runtimeDependencies = packageJson.dependencies ?? {};
  for (const dependency of forbiddenNpmRuntimeDependencies) {
    if (dependency in runtimeDependencies) {
      addFinding(`package.json [runtime-dependency] ${dependency} は実行時外部通信の再確認が必要です`);
    }
  }
}

function checkCargoToml() {
  const cargoToml = readFileSync(join(repoRoot, "src-tauri", "Cargo.toml"), "utf8");
  for (const dependency of forbiddenCargoDependencies) {
    const dependencyPattern = new RegExp(`^${escapeRegExp(dependency)}\\s*=`, "m");
    if (dependencyPattern.test(cargoToml)) {
      addFinding(`src-tauri/Cargo.toml [runtime-dependency] ${dependency} は実行時外部通信の再確認が必要です`);
    }
  }
}

function checkTauriConfig() {
  const configPath = join(repoRoot, "src-tauri", "tauri.conf.json");
  const config = JSON.parse(readFileSync(configPath, "utf8"));
  const csp = config.app?.security?.csp ?? "";

  if (/\bhttps?:|[*]/.test(csp)) {
    addFinding("src-tauri/tauri.conf.json [csp] CSPにリモートオリジンまたはワイルドカードが含まれています");
  }
  if (config.bundle?.createUpdaterArtifacts !== false) {
    addFinding("src-tauri/tauri.conf.json [updater] createUpdaterArtifacts は false にしてください");
  }
}

function checkCapabilities() {
  const capabilitiesDir = join(repoRoot, "src-tauri", "capabilities");
  if (!existsSync(capabilitiesDir)) {
    addFinding("src-tauri/capabilities [missing] Tauri capability定義が見つかりません");
    return;
  }

  for (const filePath of walkFiles(capabilitiesDir)) {
    if (extname(filePath) !== ".json") {
      continue;
    }
    const relativePath = relative(repoRoot, filePath);
    const capability = JSON.parse(readFileSync(filePath, "utf8"));
    const permissions = capability.permissions ?? [];
    for (const permission of permissions) {
      if (typeof permission !== "string") {
        addFinding(`${relativePath} [permission] 文字列以外のpermissionは手動レビューしてください`);
        continue;
      }
      if (forbiddenCapabilityFragments.some((fragment) => permission.includes(fragment))) {
        addFinding(`${relativePath} [permission] ${permission} は外部通信/外部起動権限の再確認が必要です`);
      }
    }
  }
}

function checkMacOSEntitlements() {
  const entitlementsPath = join(repoRoot, "src-tauri", "Entitlements.plist");
  if (!existsSync(entitlementsPath)) {
    return;
  }
  const entitlements = readFileSync(entitlementsPath, "utf8");
  if (/com\.apple\.security\.network\.(client|server)/.test(entitlements)) {
    addFinding("src-tauri/Entitlements.plist [network] macOS network entitlementが含まれています");
  }
}

function walkFiles(dir) {
  const result = [];
  for (const entry of readdirSync(dir)) {
    const filePath = join(dir, entry);
    const stat = statSync(filePath);
    if (stat.isDirectory()) {
      result.push(...walkFiles(filePath));
    } else if (stat.isFile()) {
      result.push(filePath);
    }
  }
  return result;
}

function addFinding(message) {
  findings.push(message);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

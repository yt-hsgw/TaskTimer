#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const releaseWorkflowPath = join(rootDir, ".github", "workflows", "release.yml");
const allowedEntries = new Map([
  ["Windows", { platform: "windows-latest", rustTarget: "", args: "" }],
  [
    "macOS Apple Silicon",
    {
      platform: "macos-latest",
      rustTarget: "aarch64-apple-darwin",
      args: "--target aarch64-apple-darwin",
    },
  ],
  [
    "macOS Intel",
    {
      platform: "macos-latest",
      rustTarget: "x86_64-apple-darwin",
      args: "--target x86_64-apple-darwin",
    },
  ],
]);

function fail(message) {
  throw new Error(message);
}

function validateMatrices(matrices) {
  for (const matrix of matrices) {
    if (!Array.isArray(matrix.include)) {
      fail("Release build matrix.includeは配列で指定してください");
    }
    for (const entry of matrix.include) {
      const expected = allowedEntries.get(entry.label);
      if (!expected) {
        fail(`未承認のRelease artifactターゲットです: ${entry.label ?? "(labelなし)"}`);
      }
      for (const field of ["platform", "rustTarget", "args"]) {
        if (entry[field] !== expected[field]) {
          fail(
            `${entry.label}.${field}が承認済み設定と一致しません: ${entry[field] ?? "(未設定)"}`,
          );
        }
      }
    }
  }

  const matrixLabels = matrices
    .map((matrix) => matrix.include.map((entry) => entry.label).sort().join("|"))
    .sort();
  const expectedLabels = [
    "Windows",
    "Windows|macOS Apple Silicon|macOS Intel",
  ].sort();
  if (JSON.stringify(matrixLabels) !== JSON.stringify(expectedLabels)) {
    fail(`Release build matrixの構成が不正です: ${matrixLabels.join(", ")}`);
  }
}

function verifyUnsupportedTargetIsRejected() {
  try {
    validateMatrices([
      { include: [{ label: "Linux", platform: "ubuntu-latest", rustTarget: "", args: "" }] },
    ]);
  } catch (error) {
    if (error.message.includes("未承認のRelease artifactターゲット")) {
      return;
    }
    throw error;
  }
  fail("Linux artifactを拒否する否定系テストに失敗しました");
}

try {
  verifyUnsupportedTargetIsRejected();

  const workflow = readFileSync(releaseWorkflowPath, "utf8");
  const matrixMatches = [...workflow.matchAll(/echo 'build_matrix=([^']+)'/g)];
  if (matrixMatches.length !== 2) {
    fail(`Release build matrixは2種類必要です: detected=${matrixMatches.length}`);
  }

  const matrices = matrixMatches.map((match) => {
    try {
      return JSON.parse(match[1]);
    } catch (error) {
      fail(`Release build matrixをJSONとして解析できません: ${error.message}`);
    }
  });
  validateMatrices(matrices);

  console.log("Release artifact対象: Windows、署名・公証済みmacOSのみ");
  console.log("Linuxおよび未知のartifactターゲットは含まれていません。");
} catch (error) {
  console.error(`::error title=release platform policy failed::${error.message}`);
  process.exit(1);
}

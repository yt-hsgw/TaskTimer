# 018: macOS署名・公証preflightを追加する

GitHub Issue: #24

Status: 2026-08-05にクローズ。現時点ではApple署名・公証とmacOS公式配布を進めないため、preflight script、成果物検証script、Release workflowのmacOS署名検査、npm scriptは削除済み。

## 現在方針

- Release workflowはWindows artifactのみを生成する。
- macOS artifactは現時点の配布対象外とする。
- Apple証明書、Apple ID、App用パスワード、Team IDなどのSecretsをGitHub Actionsへ要求しない。
- macOS公式配布を再開する場合は、新しいIssueでApple Developer Program、GitHub Secrets、署名・公証、Gatekeeper実機確認、成果物検証を改めて設計する。

## 削除した境界

- `scripts/check-macos-signing-preflight.mjs`
- `scripts/verify-macos-release-artifacts.mjs`
- `scripts/verify-macos-release-artifacts.test.mjs`
- `npm run check:macos-signing`
- `npm run check:macos-signing-config`
- `npm run verify:macos-release-artifacts`
- `npm run test:release-scripts`

## 設計理由

ユーザーの当面の利用環境はWindowsであり、Apple署名・公証を現在運用へ残すとRelease workflow、Secrets管理、CI検査、手動確認の責務が増える。使わない署名経路を残すより、WindowsのみのRelease境界へ単純化し、将来必要になった時点で新Issueとして再設計する方が安全である。

## トレードオフ

- macOS artifactをすぐ配布できなくなるが、現在の公開対象とCI責務が明確になる。
- 将来macOS配布を再開する際は再実装が必要だが、古いSecrets名や検証手順を誤用するリスクを避けられる。

## 破綻シナリオ

- 古いRelease手順を参照して、存在しないApple署名npm scriptを実行しようとする。
- macOS artifactを署名・公証なしで公開してしまう。
- Apple関連SecretsをIssue、PR、Release notes、Actionsログへ書いてしまう。

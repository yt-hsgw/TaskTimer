# 014: macOS署名と公証を設定する

GitHub Issue: #24

Status: 2026-08-05にクローズ。現時点ではApple署名・公証を進めず、macOS公式配布もスコープ外とする。

## 現在方針

- v0.1.xの通常ReleaseはWindows artifactのみを配布対象にする。
- Release workflowからmacOS matrix、Apple署名Secrets検証、macOS成果物検証を削除する。
- Apple署名・公証用のnpm scriptと検証scriptは保持しない。
- macOS公式配布を再開する場合は、新しいIssueを作成し、Apple Developer Program、GitHub Secrets、署名・公証、Gatekeeper実機確認、Release notes表現を改めて設計する。

## 設計理由

現在の主利用環境はWindowsであり、Apple署名・公証を維持すると、使わないRelease経路とSecrets境界をCIに残すことになる。不要な運用面を削ることで、Release workflowをWindowsに集中させ、誤って未準備のmacOS artifactを公開するリスクを下げる。

## トランザクション境界

- Release workflow: Windows artifactを生成し、Draft Releaseへ添付する境界。
- macOS公式配布再開: 将来Issueで再設計する境界。現在のworkflowとは分離する。

## セキュリティ

- Apple証明書、Apple ID、App用パスワード、Team IDなどのSecretsを現在のworkflowへ要求しない。
- Apple関連Secretsをリポジトリ、Issue、PR、Release notes、Actionsログへ書かない。
- macOS artifactを配布する場合は、署名・公証・Gatekeeper実機確認なしで公開しない。

## トレードオフ

- macOS利用者向けの公式配布は遅れる。
- 現在のCIとRelease手順は単純になり、Windows利用者へ届ける作業の失敗点が減る。
- 将来macOS配布を再開する場合は再実装が必要になるが、その時点のTauri、Apple、GitHub Actions仕様に合わせて設計できる。

## 代替案

Apple署名・公証scriptだけを残し、workflowから外す。

不採用理由:

- 実行されないscriptが残ると、古い手順やSecrets名を正として誤用する可能性がある。

macOS未署名DMGを配布する。

不採用理由:

- Gatekeeper警告により一般利用者が安全に判断しづらく、公開配布品質として扱いにくい。

## 破綻シナリオ

- macOS公式配布を再開する判断をIssue化せず、古いRelease手順で未検証artifactを公開する。
- Apple関連SecretsをGitHub本文やActionsログへ出してしまう。
- Windowsコード署名の課題とmacOS署名・公証の課題を混同する。

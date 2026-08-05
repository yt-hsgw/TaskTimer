# Apple署名CI削除レビュー

## 対象

- Apple署名・公証preflight scriptの削除
- macOS成果物検証scriptの削除
- Release workflowのWindows単独化
- リリースチェックとテスト戦略の現行運用への更新
- Windows hosted runnerのUI smoke初期表示しきい値調整

## 仕様

現時点ではApple署名・公証とmacOS公式配布を扱わない。通常ReleaseはWindows artifactのみを生成し、macOS配布を再開する場合は新Issueで再設計する。

## 設計理由

使わないApple署名経路をCIとnpm scriptに残すと、Secrets管理、成果物検証、手動確認の責務が現行Releaseに残る。Windowsを主対象にする現在運用では、Release境界をWindowsのみに絞る方が失敗点と誤公開リスクを減らせる。

Windows hosted runnerのUI smokeでは、Vite起動後の初回描画が10秒前後まで揺れるケースがある。初期表示だけはcold start、Chrome起動、runner負荷の影響を受けやすいため、Windowsの `initial_task_list` しきい値を12秒に上げる。以降の画面遷移、D&D、保存、検索などのしきい値は据え置き、実操作の回帰検知は維持する。

## トランザクション境界

- Release workflow: Windows artifactをDraft Releaseへ添付する境界。
- Apple署名・公証: 現在のworkflow外。将来Issueで再定義する境界。

## セキュリティ

- Apple証明書、Apple ID、App用パスワード、Team IDなどのSecretsを現在workflowへ要求しない。
- Apple関連SecretsをIssue、PR、Release notes、Actionsログへ書かない。
- macOS artifactを再開する場合は、署名・公証・Gatekeeper実機確認が完了するまで公開しない。

## トレードオフ

- macOS公式配布の再開時には検証scriptとworkflowを再実装する必要がある。
- 現在のReleaseはWindowsに集中でき、CIと手動確認が短くなる。
- 初期表示のWindowsしきい値は緩くなるが、runnerのcold start揺れによる偽陽性を減らせる。

## 代替案

Apple署名scriptだけ残してworkflowから外す。

不採用理由:

- 実行されないscriptが古いSecrets名や検証条件の正として残り、将来の誤用につながる。

## 破綻シナリオ

- 古いドキュメントを見て存在しないApple署名scriptを実行する。
- macOS artifactを署名・公証なしに公開する。
- Windowsコード署名の制限とApple署名・公証の制限を混同する。

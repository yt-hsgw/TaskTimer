# Issue草案

このディレクトリは、GitHub Issue化した作業や、Issue化前の設計メモを残す場所です。

GitHub上のIssueを正とし、このディレクトリは設計理由、トレードオフ、危険ケースを後から追える補助資料として扱います。

## 草案一覧

- [001 SQLite接続とRepository境界を実装する](001-sqlite-connection-repository-boundary.md)
- [002 コアUse Caseを実装する](002-core-usecases.md)
- [003 UIをDB接続へ切り替える](003-connect-ui-to-db.md)
- [004 タイマー完了・削除・復元の境界ケースを固める](004-timer-delete-edge-cases.md)
- [005 ローカル通知を実装する](005-local-notification.md)
- [006 リリース運用とGitHub管理を整える](006-release-and-github-operations.md)
- [007 外部利用者向けGitHub運用](007-public-user-operations.md)

## 運用

1. GitHub IssueまたはPRに作業単位を作る。
2. 非自明な判断は、このディレクトリまたは `docs/adr` に理由、トレードオフ、代替案を残す。
3. 実装後はGitHub Issue/PRのリンクを必要に応じて追記する。

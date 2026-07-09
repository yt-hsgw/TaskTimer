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
- [008 UI/UX再設計用データモデルとRead Modelを整備する](008-ui-data-read-model.md) / GitHub #27
- [009 左ナビゲーションとApp Shellを実装する](009-ui-app-shell-navigation.md) / GitHub #26
- [010 タスク一覧を新UIへ置き換える](010-ui-task-list-redesign.md) / GitHub #25
- [011 右詳細ペインを実装する](011-ui-task-detail-pane.md) / GitHub #29
- [012 カレンダーと設定を左ナビ配下のビューへ移管する](012-ui-calendar-settings-migration.md) / GitHub #28
- [013 タイマー一時停止/再開と繰り返し設定を設計・実装する](013-timer-recurrence-detail-extensions.md) / GitHub #30
- [014 macOS署名と公証を設定する](014-release-macos-signing-notarization.md) / GitHub #24

## 運用

1. GitHub IssueまたはPRに作業単位を作る。
2. 非自明な判断は、このディレクトリまたは `docs/adr` に理由、トレードオフ、代替案を残す。
3. 実装後はGitHub Issue/PRのリンクを必要に応じて追記する。

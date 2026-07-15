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
- [015 v0.1.0公開判定資料を整える](015-release-v010-readiness.md) / GitHub #20
- [016 glib advisory監視を自動化する](016-glib-advisory-watch.md) / GitHub #22
- [017 v0.1.0のRelease target検証を追加する](017-release-target-guard.md) / GitHub #20
- [018 macOS署名・公証preflightを追加する](018-macos-signing-preflight.md) / GitHub #24
- [019 次の作業リストを現状に合わせて更新する](019-next-actions-refresh.md) / GitHub #20
- [020 Windows優先Release workflowへ切り替える](020-windows-first-release-workflow.md) / GitHub #20/#24
- [021 Windows runnerでインストーラー最低限検証を追加する](021-windows-installer-runner-smoke.md) / GitHub #20
- [022 カレンダー週/日/月ビューと時間軸表示を実装する](022-calendar-view-modes-time-grid.md) / GitHub #60
- [023 実行時外部通信・ログ出力の静的監査を追加する](023-runtime-privacy-audit.md) / GitHub #49
- [024 Windowsコード署名方針を決める](024-windows-code-signing-policy.md) / GitHub #50
- [025 OSスリープ・復帰時のタイマーと通知を強化する](025-sleep-resume-timer-notification.md) / GitHub #58
- [026 通知の全体有効・無効設定を追加する](026-notification-rule-toggle-ui.md) / GitHub #55（完了）
- [027 タスク詳細とサブタスク選択UXを改善する](027-task-detail-subtask-ux.md) / GitHub #68
- [028 大量データで一覧とカレンダー表示を検証する](028-performance-large-dataset.md) / GitHub #72
- [029 ローカルデータのバックアップとエクスポート方針を設計する](029-data-backup-export.md) / GitHub #73
- [030 通知失敗履歴と再試行結果を表示する](030-notification-failure-history.md) / GitHub #53
- [031 カスタムリスト管理を追加する](031-custom-list-management.md) / GitHub #54
- [032 JSON/CSVエクスポートUse Caseを実装する](032-json-csv-export-usecase.md) / GitHub #87
- [033 SQLiteバックアップ/復元Use Caseを実装する](033-sqlite-backup-restore-usecase.md) / GitHub #88
- [034 バックアップ/復元/エクスポートUIを追加する](034-data-management-settings-ui.md) / GitHub #89
- [035 Rust静的解析CIの実行時間を短縮する](035-rust-ci-optimization.md) / GitHub #94
- [036 タスクのアーカイブ操作をUse Caseへ追加する](036-task-archive-usecase.md) / GitHub #52
- [037 UI設定の永続化範囲を拡張する](037-ui-preferences-persistence.md) / GitHub #57

## 運用

1. GitHub IssueまたはPRに作業単位を作る。
2. 非自明な判断は、このディレクトリまたは `docs/adr` に理由、トレードオフ、代替案を残す。
3. 実装後はGitHub Issue/PRのリンクを必要に応じて追記する。

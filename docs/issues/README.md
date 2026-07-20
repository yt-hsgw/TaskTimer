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
- [016 glib advisory監視を自動化する](016-glib-advisory-watch.md) / GitHub #22（完了）
- [017 v0.1.0のRelease target検証を追加する](017-release-target-guard.md) / GitHub #20
- [018 macOS署名・公証preflightを追加する](018-macos-signing-preflight.md) / GitHub #24
- [019 次の作業リストを現状に合わせて更新する](019-next-actions-refresh.md) / GitHub #20
- [020 Windows優先Release workflowへ切り替える](020-windows-first-release-workflow.md) / GitHub #20/#24
- [021 Windows runnerでインストーラー最低限検証を追加する](021-windows-installer-runner-smoke.md) / GitHub #20
- [022 カレンダー週/日/月ビューと時間軸表示を実装する](022-calendar-view-modes-time-grid.md) / GitHub #60
- [023 実行時外部通信・ログ出力の静的監査を追加する](023-runtime-privacy-audit.md) / GitHub #49
- [024 Windowsコード署名方針を決める](024-windows-code-signing-policy.md) / GitHub #50
- [025 OSスリープ・復帰時のタイマーと通知を強化する](025-sleep-resume-timer-notification.md) / GitHub #58（完了）
- [026 通知の全体有効・無効設定を追加する](026-notification-rule-toggle-ui.md) / GitHub #55（完了）
- [027 タスク詳細とサブタスク選択UXを改善する](027-task-detail-subtask-ux.md) / GitHub #68
- [028 大量データで一覧とカレンダー表示を検証する](028-performance-large-dataset.md) / GitHub #72（完了）
- [029 ローカルデータのバックアップとエクスポート方針を設計する](029-data-backup-export.md) / GitHub #73
- [030 通知失敗履歴と再試行結果を表示する](030-notification-failure-history.md) / GitHub #53
- [031 カスタムリスト管理を追加する](031-custom-list-management.md) / GitHub #54
- [032 JSON/CSVエクスポートUse Caseを実装する](032-json-csv-export-usecase.md) / GitHub #87
- [033 SQLiteバックアップ/復元Use Caseを実装する](033-sqlite-backup-restore-usecase.md) / GitHub #88
- [034 バックアップ/復元/エクスポートUIを追加する](034-data-management-settings-ui.md) / GitHub #89
- [035 Rust静的解析CIの実行時間を短縮する](035-rust-ci-optimization.md) / GitHub #94
- [036 タスクのアーカイブ操作をUse Caseへ追加する](036-task-archive-usecase.md) / GitHub #52
- [037 UI設定の永続化範囲を拡張する](037-ui-preferences-persistence.md) / GitHub #57
- [038 カレンダーからタスクを追加できるようにする](038-calendar-task-create.md) / GitHub #82
- [039 カレンダー上でタスク期限を調整できるようにする](039-calendar-reschedule.md) / GitHub #83
- [040 カレンダー上のタスク色を変更できるようにする](040-calendar-list-colors.md) / GitHub #84
- [041 タスクにタグを付けられるようにする](041-task-tags.md) / GitHub #80
- [042 かんばん形式の画面を追加する](042-kanban-board.md) / GitHub #81
- [043 ポモドーロタイマーを設計・実装する](043-pomodoro-timer.md) / GitHub #107
- [044 OSへの将来時刻通知スケジューリング方式を設計する](044-notification-future-scheduling.md) / GitHub #51
- [045 アプリ起動中の将来時刻通知スケジューラを実装する](045-notification-in-app-scheduler.md) / GitHub #116
- [046 起動・復帰・設定変更時の通知再同期を実装する](046-notification-resync-events.md) / GitHub #117
- [047 通知OS登録状態のRepository境界とDB状態を追加する](047-notification-os-registration-state.md) / GitHub #115
- [048 Windows/macOSネイティブ将来通知adapterの実現性を検証する](048-native-notification-adapter-feasibility.md) / GitHub #118
- [049 Windowsネイティブ将来通知adapterのPoCを実装する](049-windows-native-notification-poc.md) / GitHub #123
- [050 作業画面の操作配置と表示密度を整理する](050-ui-workspace-polish.md)
- [051 かんばんをドラッグ操作とカスタム状態へ拡張する](051-kanban-custom-workflow.md) / GitHub #126
- [052 カレンダー項目を期間表示し端のドラッグで調整する](052-calendar-duration-resize.md) / GitHub #127
- [053 タスク一覧の200件上限をページングで解消する](053-task-list-pagination.md) / GitHub #131（完了）
- [054 かんばんカード全面のドラッグ操作と前面表示を改善する](054-kanban-full-card-drag-overlay.md) / GitHub #138
- [055 カレンダーの予定ブロック移動と期限調整操作を統合する](055-calendar-block-move-and-due-edit.md) / GitHub #139
- [056 操作後の全画面再取得と再描画を表示範囲単位へ分割する](056-scoped-ui-refresh.md) / GitHub #140
- [057 ポモドーロをタスクから独立した集中機能へ再設計する](057-standalone-pomodoro.md) / GitHub #141
- [058 タスク行へカウントダウンタイマーと完了通知を追加する](058-task-countdown-timer.md) / GitHub #142
- [059 カレンダーのドラッグ範囲から予定付きタスクを作成する](059-calendar-drag-range-create.md) / GitHub #146
- [060 カレンダーの重複予定を横並び表示する](060-calendar-overlap-layout.md) / GitHub #147
- [061 カレンダーの省略予定を一覧表示する](061-calendar-overflow-popover.md) / GitHub #148
- [062 カレンダーの時間グリッド表示回帰を修正する](062-calendar-time-grid-regression.md) / GitHub #152
- [063 ワークスペース表示切り替えとローカル検索を実装する](063-workspace-navigation-local-search.md) / GitHub #157
- [064 親タスク作成を共通ダイアログへ統合する](064-unified-task-create-dialog.md) / GitHub #158
- [065 タスク表示色をリスト色から分離する](065-task-display-color.md) / GitHub #159
- [066 タイムライン表示を実装する](066-timeline-view.md) / GitHub #160
- [067 詳細画面と左ペインの管理操作を整理する](067-detail-navigation-cleanup.md) / GitHub #161
- [068 左ペインのリスト管理操作を縦三点メニューへ集約する](068-list-actions-overflow-menu.md) / GitHub #163
- [069 今日ビューから開始予定日付きタスクを追加する](069-today-task-create.md)
- [070 左ペインとカンバンの操作配置を整理する](070-navigation-kanban-action-layout.md) / GitHub #178
- [071 日時未設定タスクをカレンダー／タイムラインへD&Dして予定化する](071-assign-unscheduled-work-schedule.md) / GitHub #181
- [072 かんばんの未設定タスクをD&Dで予定化する](072-kanban-unscheduled-work-schedule.md) / GitHub #182

## 運用

1. GitHub IssueまたはPRに作業単位を作る。
2. 非自明な判断は、このディレクトリまたは `docs/adr` に理由、トレードオフ、代替案を残す。
3. 実装後はGitHub Issue/PRのリンクを必要に応じて追記する。

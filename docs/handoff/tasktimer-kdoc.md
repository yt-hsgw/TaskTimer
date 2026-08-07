# TaskTimer KDoc風リファレンス

この資料は、KotlinのKDocに近い粒度で、TaskTimerの主要モジュール、責務、不変条件、副作用、参照先を読むための引き継ぎリファレンスです。

実装言語はTypeScript/Rustですが、次のAIが構造を拾いやすいように、次のタグを使います。

- `@layer`: Clean Architecture上の所属レイヤー。
- `@file`: 主な実装ファイル。
- `@responsibility`: その単位が持つ責務。
- `@invariant`: 破ってはいけない不変条件。
- `@transaction`: DBトランザクション境界。
- `@sideEffect`: DBコミット後またはUI上の副作用。
- `@security`: セキュリティ/プライバシー上の注意。
- `@aiNote`: 次のAIへの補足。
- `@see`: 関連資料や関連実装。

## Project

```kotlin
/**
 * オフライン対応TODO・タイマー管理デスクトップアプリ。
 *
 * @layer Product
 * @responsibility タスク、サブタスク、通常タイマー、独立ポモドーロ、カレンダー予定、通知、エクスポートをローカルで扱う。
 * @invariant アプリ実行時の外部通信を追加しない。
 * @invariant 通常タイマーとポモドーロを合わせ、アクティブな作業は同時に1件だけ。
 * @security タスク名、サブタスク名、メモ本文、通知本文をログへ出さない。
 * @security ユーザー本文をHTMLとして描画しない。
 * @aiNote 予定期間 scheduled_* と期限 due_* は別概念。カレンダーD&Dで通知期限を誤更新しないこと。
 * @see docs/handoff/tasktimer-structure.json
 * @see docs/handoff/tasktimer-blueprint.html
 * @see docs/architecture.md
 */
object TaskTimer
```

## AppShell

```kotlin
/**
 * TaskTimerのPresentation状態を束ねるアプリケーションシェル。
 *
 * @layer Presentation
 * @file src/presentation/App.tsx
 * @responsibility 左ペイン、中央ビュー、右詳細、検索、通知同期、タイマー同期、共通タスク作成ダイアログを統合する。
 * @responsibility MutationごとにReadModelRefreshPlanを選び、必要なRead Modelだけを再取得する。
 * @invariant チェック、お気に入り、タイマー操作などの局所更新で画面全体をローディング状態へ戻さない。
 * @invariant 右詳細ペインの開閉は選択操作に限定し、チェック/お気に入り/期限更新/予定移動の副作用で自動表示しない。
 * @sideEffect 通知再同期、ポモドーロ期限同期、通常タイマー期限同期をUIスナップショット更新の周辺で実行する。
 * @aiNote UIのちらつき調査では、まずMutationScopeとReadModelRefreshPlanを確認する。
 * @see src/presentation/renderProbe.ts
 * @see src/application/usecases/contracts.ts
 */
class AppShell
```

## LeftNavigation

```kotlin
/**
 * ワークスペース選択とリスト管理の入口。
 *
 * @layer Presentation
 * @file src/presentation/components/LeftNavigation.tsx
 * @responsibility 今日、お気に入り、リスト一覧、設定への導線を表示する。
 * @responsibility リスト追加、リスト名編集、リスト三点メニューを扱う。
 * @invariant 初期リスト「タスク」は全タスクを表示するスマート範囲であり、通常の所属先候補として扱わない。
 * @invariant 新規リスト名が空白なら作成しない。
 * @invariant アクティブなリストは初期リストを含めて最大10件。
 * @aiNote リスト編集は保存ボタンではなく、Enterまたはフォーカスアウト保存が基本。
 * @see docs/ui-ux-redesign.md
 */
class LeftNavigation
```

## TaskPanel

```kotlin
/**
 * リスト/今日/お気に入り/タグなどのタスク一覧ビュー。
 *
 * @layer Presentation
 * @file src/presentation/components/TaskPanel.tsx
 * @responsibility 未完了タスク、完了セクション、サブタスク展開、行メニュー、ページングを表示する。
 * @responsibility タスク作成ダイアログの開始、完了切替、お気に入り切替、行選択を発火する。
 * @invariant チェックボタンとお気に入りボタン押下では右詳細を開かない。
 * @invariant 完了済みタスクは訂正線と半透明で表示し、完了セクションへ移動する。
 * @invariant 親タスクにサブタスクがある場合だけ進捗バーと数値を表示する。
 * @aiNote 行本文クリックと行内ボタンのイベント伝播を混同しない。
 * @see src-tauri/src/application/usecases.rs#list_task_page
 */
class TaskPanel
```

## TaskDetailPane

```kotlin
/**
 * タスクまたはサブタスクの詳細オーバーレイ。
 *
 * @layer Presentation
 * @file src/presentation/components/TaskDetailPane.tsx
 * @responsibility タイトル、タグ、所属リスト、タスク色、期限、目標時間、繰り返し、サブタスク一覧、メモ、削除を編集する。
 * @responsibility 表示状態からインライン編集へ切り替え、Enterまたはフォーカスアウトで保存する。
 * @invariant 空白タイトルは保存せず編集前へ戻す。
 * @invariant タグ名編集は同じタグを使う他タスクにも反映される。
 * @invariant サブタスク詳細では親タスクのタグを継承表示し、直接タグ編集はしない。
 * @invariant 期限日/期限時刻の設定で右詳細を再オープンしない。
 * @sideEffect 領域外クリックでポップアップを閉じる。別の設定値入力開始時は既存ポップアップを閉じる。
 * @aiNote 期限 due_* と予定 scheduled_* はこの詳細内でも別の編集領域として扱う。
 * @see docs/ui-ux-redesign.md
 */
class TaskDetailPane
```

## TaskCreateDialog

```kotlin
/**
 * 親タスク作成の共通ダイアログ。
 *
 * @layer Presentation
 * @file src/presentation/components/TaskCreateDialog.tsx
 * @responsibility リスト、今日、かんばん、カレンダー範囲作成からの親タスク追加UIを共通化する。
 * @invariant サブタスク追加UIとは完全には統合しない。親タスク作成の共通入口として扱う。
 * @invariant 作成後に右詳細ペインを自動で開かない。
 * @transaction 通常作成は create_task、状態列作成は create_task_in_board_column、予定付き作成は create_scheduled_task。
 * @aiNote カレンダー作成では選択範囲をscheduled_*へ保存し、期限 due_* は自動設定しない。
 * @see src/presentation/taskCreate.ts
 */
class TaskCreateDialog
```

## KanbanBoard

```kotlin
/**
 * 状態列ごとのタスク管理ビュー。
 *
 * @layer Presentation
 * @file src/presentation/components/KanbanBoard.tsx
 * @responsibility 状態列追加、列名編集、列D&D並べ替え、カードD&D状態変更、列内タスク追加を扱う。
 * @invariant タスクカードはカード全面をドラッグ起点にする。
 * @invariant ドラッグ中のカードは前面オーバーレイへ出し、移動先列の裏に隠さない。
 * @invariant ドロップ後は保存完了まで移動先へ楽観表示し、移動元カードを一瞬再表示しない。
 * @invariant 完了済みタスクは元の状態列の下部にある完了セクションへ表示する。
 * @transaction 状態変更は move_task_to_board_column または update_task_status を経由する。
 * @aiNote 列ソートは表示切り替えでありDB順序を変更しない。
 * @see docs/issues/042-kanban-board.md
 */
class KanbanBoard
```

## WeekCalendar

```kotlin
/**
 * 週/日/月を持つGoogleカレンダー型の予定ビュー。
 *
 * @layer Presentation
 * @file src/presentation/components/WeekCalendar.tsx
 * @responsibility カレンダー表示、ドラッグ範囲作成、予定移動、予定リサイズ、重複レーン、他N件ポップアップを扱う。
 * @invariant 日/週の時刻あり予定は15分単位、月/終日予定は1日単位。
 * @invariant 日をまたぐ予定は上部予定行へ表示する。
 * @invariant 複数日予定は月表示で連続バーとして表示し、週をまたぐ場合だけ分割する。
 * @invariant 予定ブロック本体のD&Dは期間長と終日状態を維持する。
 * @transaction 予定移動は move_scheduled_work_item、予定リサイズは resize_scheduled_work_item、未設定割り当ては assign_work_schedule。
 * @aiNote due_* の期限マーカー移動と scheduled_* の予定移動を混ぜない。
 * @see docs/issues/055-calendar-block-move-and-due-edit.md
 * @see docs/issues/071-assign-unscheduled-work-schedule.md
 */
class WeekCalendar
```

## PomodoroPanel

```kotlin
/**
 * 独立ポモドーロの集中画面。
 *
 * @layer Presentation
 * @file src/presentation/components/PomodoroPanel.tsx
 * @responsibility 作業/短休憩/長休憩、残り時間、セット数、円形残量ディスク、操作ボタンを表示する。
 * @invariant 状態によりボタン構成が横ズレしないよう固定スロットを維持する。
 * @invariant ボタン操作で他ボタンや画面全体をちらつかせない。
 * @transaction start_standalone_pomodoro、pause_pomodoro、resume_pomodoro、complete_pomodoro_work_phase、start_pomodoro_break、skip_pomodoro_break、complete_pomodoro_break、cancel_pomodoro。
 * @sideEffect 作業完了時はローカル通知を行う。
 * @aiNote 通常タイマーとは別機能だが、アクティブ制約は共有する。
 * @see docs/issues/057-standalone-pomodoro.md
 */
class PomodoroPanel
```

## SettingsPanel

```kotlin
/**
 * ローカル設定とエクスポートの画面。
 *
 * @layer Presentation
 * @file src/presentation/components/SettingsPanel.tsx
 * @responsibility 通知ON/OFF、通知表示タイプ、JSON/CSVエクスポート、ポモドーロ既定値、通常タイマー既定値を扱う。
 * @invariant 通知表示タイプはカード型ラジオ選択。
 * @invariant 通知失敗履歴は通常設定画面へ表示しない。
 * @invariant SQLiteバックアップ/復元は通常設定画面へ表示しない。JSON/CSVエクスポートを主導線にする。
 * @security エクスポート失敗時のログにタスク名、メモ本文、通知本文を含めない。
 * @see docs/data-backup-export.md
 */
class SettingsPanel
```

## ApplicationUseCases

```kotlin
/**
 * DBトランザクション境界を持つアプリケーション操作群。
 *
 * @layer Application
 * @file src-tauri/src/application/usecases.rs
 * @responsibility 入力検証、存在確認、Domain policy適用、Repository呼び出し、DBコミット後副作用の準備を行う。
 * @invariant Presentationはトランザクション仕様を決めない。
 * @invariant OS通知送信、アプリ起動中タイマー予約、Windowsネイティブ通知登録はDBトランザクションに含めない。
 * @transaction create_task/update_task/delete_task/create_subtask/update_subtask/delete_subtask/list_task_page/list_calendar_items など。
 * @security Notification DTOやスケジューラ用DTOへタスク名、サブタスク名、メモ本文、通知本文を不要に含めない。
 * @aiNote 新しいUse Caseを追加したら、commands.rs、dto.rs、contracts.ts、gateway.ts、docs/architecture.mdを確認する。
 * @see src-tauri/src/application/commands.rs
 * @see src-tauri/src/application/repositories.rs
 * @see src/application/usecases/contracts.ts
 */
object ApplicationUseCases
```

## TauriCommands

```kotlin
/**
 * Reactから呼び出されるTauri command境界。
 *
 * @layer Application
 * @file src-tauri/src/application/commands.rs
 * @responsibility Tauri StateからDatabase/Clock/Notification adapterを受け取り、Use Caseへ委譲する。
 * @invariant command内に業務ルールを増やさない。
 * @invariant エラーは文字列としてUIへ返るため、ユーザー本文や秘密情報を含めない。
 * @aiNote command追加時はフロントのgatewayとcontractsも同時に更新する。
 * @see src/infrastructure/tauri/gateway.ts
 */
object TauriCommands
```

## SQLiteRepository

```kotlin
/**
 * Repository portのSQLite実装。
 *
 * @layer Infrastructure
 * @file src-tauri/src/infrastructure/sqlite.rs
 * @responsibility Read Model取得、書き込みトランザクション、ソフト削除、マイグレーション適用を実装する。
 * @invariant アクティブタイマーはSQLiteの一意制約 one_active_timer でも守る。
 * @invariant 予定期間取得は表示範囲で絞る。
 * @invariant 削除は原則 deleted_at を使う。
 * @security SQLへユーザー入力を直接連結しない。
 * @aiNote docs/database-schema.sqlは設計上の正、src-tauri/migrations/0001_initial.sqlは実行時の正。変更時は両方確認する。
 * @see docs/database-schema.sql
 * @see src-tauri/migrations/0001_initial.sql
 */
class SqliteDatabase
```

## NotificationBoundary

```kotlin
/**
 * 通知意図とOS通知副作用の境界。
 *
 * @layer Application/Infrastructure
 * @file src-tauri/src/application/notification.rs
 * @file src-tauri/src/infrastructure/notification.rs
 * @responsibility notification_rulesを通知意図の正とし、期限到来時にOS通知を試行する。
 * @invariant OS通知送信はDBコミット後の副作用。
 * @invariant generic通知ではタスク名、サブタスク名、メモ本文を表示しない。
 * @invariant OS登録状態はnotification_os_registrationsへ分離し、notification_rulesへ混ぜない。
 * @sideEffect 送信成功/失敗を登録状態やdelivery attemptsへ記録する。
 * @security OSエラー保存時もユーザー本文を含めない。
 * @see docs/security.md
 * @see docs/issues/044-notification-future-scheduling.md
 */
object NotificationBoundary
```

## TimerBoundary

```kotlin
/**
 * 通常タイマーとポモドーロのアクティブ制約。
 *
 * @layer Domain/Application
 * @file src-tauri/src/domain/timer.rs
 * @file src-tauri/src/domain/pomodoro.rs
 * @file src-tauri/src/application/usecases.rs
 * @responsibility 通常タイマー、ポモドーロ作業、休憩、一時停止、再開、完了、終了を扱う。
 * @invariant 通常タイマーとポモドーロを合わせてアクティブ作業は1件だけ。
 * @invariant 通常タイマーの経過秒数はwall-clock差分から一時停止区間を差し引いて確定する。
 * @invariant ポモドーロ休憩はtimer_sessionsへ作業時間として混ぜない。
 * @sideEffect タスクカウントダウン完了とポモドーロ作業完了で通知を行う。
 * @see docs/adr/0003-single-active-timer.md
 * @see docs/issues/058-task-countdown-timer.md
 * @see docs/issues/057-standalone-pomodoro.md
 */
object TimerBoundary
```

## ChangeProtocol

```kotlin
/**
 * 変更時に守る順序。
 *
 * @layer Operations
 * @responsibility 仕様、設計、レビュー、実装、検証の順に進める。
 * @invariant 非自明な変更ではdocs/review/checklist.mdを確認する。
 * @invariant 仕様や前提が変わる場合はdocs/handoff/tasktimer-structure.jsonとこのKDoc風資料も更新対象にする。
 * @aiNote 次のAIはまずJSONを読み、次にこの資料で担当領域の責務と不変条件を見る。
 * @see AGENTS.md
 * @see docs/review/checklist.md
 */
object ChangeProtocol
```

# 次の作業

## 現在の判断

UI/UX再設計の主要Issueは完了済みです。次の主作業は、v0.1.0を外部利用者へ配布できる状態にするためのRelease前ゲートです。

GitHub上で継続追跡しているOpen Issue:

- #20: v0.1.0 Release。
- #22: Tauri経由のglib advisory追跡。
- #24: macOS署名と公証。

## 最優先

1. Apple Developer ProgramでDeveloper ID Application証明書を発行する。
2. macOS署名・公証用GitHub Actions Secretsを登録する。
3. `npm run check:macos-signing` を実行し、preflightが成功することを確認する。
4. 最終的な `main` に対して `app-v0.1.0` tagとDraft Releaseを作り直す。
5. `npm run check:release-target -- 0.1.0 origin/main` でRelease tagと公開対象commitの一致を確認する。
6. Release workflowでmacOS Apple Silicon、macOS Intel、Windowsのartifact生成が成功することを確認する。
7. macOS DMGを実機で開き、Gatekeeper警告が解消されることを確認する。
8. Windows NSISインストーラーを実機でインストール、起動、アンインストール確認する。
9. Release Issue #20へ手動確認結果、既知制限、glib advisory #22の扱いを記録する。
10. Release notesを最終化し、Draft Releaseを公開する。

## 完了済み

- SQLite接続とマイグレーション実行器の初期実装。
- 週カレンダー、アクティブタイマー、通知表示設定のRepository境界。
- `CreateTask`、`CreateSubtask`、`StartTimer`、`StopActiveTimer` のApplication Use Case。
- タスク/サブタスク作成、タイマー開始/停止のSQLiteトランザクション実装。
- 単一アクティブタイマー制約と停止時 `elapsed_seconds` 確定のテスト。
- タスク一覧とタスク詳細の読み取りRepository。
- UIのモックデータ削除とSQLite由来表示への切り替え。
- タスク/サブタスク作成フォームとタイマー開始/停止ボタンのUse Case接続。
- 週カレンダーの日付範囲DBクエリ表示。
- 未完了サブタスクがある親タスクの確認付き完了フロー。
- タスク削除時の子サブタスク、タイマー履歴、通知ルールのソフト削除。
- サブタスク削除時のタイマー履歴、通知ルールのソフト削除。
- タイマー開始中の対象削除時にアクティブタイマー検索から除外する処理。
- 通知表示モード設定の保存。
- タスク/サブタスク作成時の通知ルール作成。
- 期限到来通知のOS通知adapter送信、成功/失敗状態保存、再試行導線。
- UI/UX再設計仕様。
- UI/UX再設計のデータモデルとRead Model整備。
- 左ナビゲーション、中央ビュー、右詳細ペインのApp Shell。
- タスク一覧の円形チェック、お気に入り、完了セクション、サブタスク進捗表示。
- 右詳細ペインへのサブタスク、期限、通知、タイマー操作の移管。
- カレンダーと設定の左ナビ配下ビュー移管。
- タイマー一時停止/再開、繰り返し設定、タイマー目標時間。
- GitHub Issue、PRテンプレート、ラベル運用。
- GitHub Actionsによる設計/スキーマ/Rust/TypeScript基本チェック。
- リリース前チェックリストとリリースIssueテンプレート。
- MIT Licenseによる外部利用許諾。
- 外部利用者向けREADME、SUPPORT、CONTRIBUTING、CHANGELOG。
- GitHub Releases向け `リリースビルド` workflow。
- 公開運用方針とADR 0004。
- v0.1.0 Release notes草案。
- glib advisoryの週次監視workflow。
- Release target検証スクリプト。
- macOS署名・公証用のTauri設定、Release workflow下準備、preflight。

## post-v0.1.0改善候補

1. OSへの将来時刻スケジューリング方式を検討する。
2. 通知ルールの個別有効/無効UIを追加する。
3. 通知登録失敗時の再試行履歴表示を改善する。
4. タスクのアーカイブ操作をUse Caseへ追加する。
5. カスタムリスト管理をUIとUse Caseへ追加する。
6. UI設定の永続化範囲を拡張する。
7. Windowsコード署名方針を別Issueで検討する。
8. glib advisory #22が解消可能になったらCargo依存更新PRを作成する。

## 実務運用前に必要

1. Windows/macOSの手動確認手順を実行する。
2. 実行時に外部通信していないことを確認する。
3. ログにタスク名、メモ本文、通知本文が出ないことを確認する。
4. GitHub Actionsのチェック結果をRelease Issue #20に記録する。
5. Draft Releaseのartifact、target commit、Release notes、既知制限を確認する。
6. macOS署名・公証用GitHub Secretsを登録する。
7. macOS DMGを実機で開き、Gatekeeper警告が解消されることを確認する。
8. Windows未署名artifactのOS警告をRelease notesへ既知制限として記載する。

## 危険ケース

- macOS署名・公証Secrets未登録のままRelease workflowを実行する。
- `npm run check:macos-signing` の失敗を無視してDraft Releaseを公開する。
- 古いcommitで生成したDraft Release artifactを公開し、Release notesや手動確認結果と実artifactが食い違う。
- Gatekeeper警告が残るDMGを外部利用者向けに公開する。
- Windows未署名警告をRelease notesへ書かず、利用者がインストール可否を判断できない。
- glib advisory #22の影響範囲をRelease notesへ書かず、Linux配布対象外の判断が伝わらない。
- Issue、PR、Release notesへApple証明書、Apple ID、App用パスワード、Team ID、ローカルDB、個人タスク内容を貼ってしまう。

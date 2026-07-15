# 032: JSON/CSVエクスポートUse Caseを実装する

GitHub Issue: #87

## 目的

TaskTimerのタスク、サブタスク、タイマー履歴を、閲覧、監査、他ツール移行の補助としてJSON/CSVへエクスポートできるようにする。

## スコープ

- JSONエクスポート形式を定義する。
- CSVエクスポート形式を定義する。
- エクスポート対象に含まれる個人データをUIとdocsで明示する。
- エクスポートUse CaseとRepository境界を作る。
- CSVのエスケープをテストする。

## スコープ外

- JSON/CSVからの完全復元。
- クラウド連携。
- 暗号化。

## 設計レビュー

### データモデル

既存DBスキーマを正とし、エクスポート専用の永続フィールドは追加しない。

JSONは1ファイルに `manifest` と以下の配列を含める。

- `task_lists`
- `tasks`
- `subtasks`
- `timer_sessions`
- `timer_pauses`
- `notification_rules`
- `recurrence_rules`

CSVは `TaskTimer-export-YYYYMMDD-HHMMSS-csv/` フォルダに以下を出力する。

- `export-manifest.json`
- `task_lists.csv`: `id`, `name`, `color_token`, `sort_order`, `created_at`, `updated_at`
- `tasks.csv`
- `subtasks.csv`
- `timer_sessions.csv`
- `timer_pauses.csv`
- `notification_rules.csv`
- `recurrence_rules.csv`

manifestには `format`、`formatVersion`、`appVersion`、`createdAt`、`platform`、`compatibility`、`containsPersonalData` を記録する。`compatibility` は `viewing-and-migration-aid-not-restore` とし、完全復元用途ではないことを明示する。

### トランザクション境界

エクスポートは読み取りUse Caseとして扱う。複数テーブルを読むため、必要なら読み取りトランザクションで一貫したスナップショットを作る。

実装ではApplication Use Caseが入力パスと時刻、アプリバージョン、プラットフォームを確定し、Infrastructureが読み取りトランザクションでエクスポート対象を取得してからローカルファイルへ書き出す。ファイル出力失敗時、CSVフォルダは削除して中途半端な成果物を残さない。

### セキュリティ

JSON/CSVにはタスク名、サブタスク名、メモ本文、タイマー履歴が含まれる。Issue、PR、Discussionsへ添付しない。ログにはユーザー内容を出さない。

CSVは改行、カンマ、ダブルクォートをエスケープする。加えて、表計算ソフトで数式として解釈されるリスクを避けるため、`=`, `+`, `-`, `@` などで始まるセルはアポストロフィで安全化する。JSONは値をそのまま保持する。

### 代替案

SQLiteバックアップだけを提供する案もある。実装は少なく済むが、人間が内容を確認しづらく、監査や他ツール移行の補助にならないため不採用。

CSVに全データを1ファイルで出す案もある。ファイル数は減るが、タスク、サブタスク、タイマー履歴、通知ルールの列が混在し、表計算で扱いづらいため不採用。

### トレードオフ

- JSON/CSVは削除済み行を対象外にする。完全復元や削除済み履歴を含めた退避はSQLiteバックアップが正。
- CSVは表計算安全化のため一部セルの先頭にアポストロフィを付ける。正確な値を機械的に扱う用途ではJSONを使う。
- 大量データでは一度メモリへ読み込む。実務上の性能測定は #72 で追跡する。

## 受け入れ条件

- JSON/CSVのフィールド、個人データの扱い、互換性方針がdocsへ記録されている。
- Repository境界を通して必要データだけを取得する。
- CSVでメモ本文の改行、カンマ、ダブルクォートが壊れない。
- エクスポートファイルを公開場所へ添付しない注意がREADMEまたはdocsから辿れる。

## 危険ケース

- CSVエスケープ不備でメモ本文が壊れる。
- エクスポートファイルを公開Issueへ添付してしまう。
- JSON/CSVが完全復元できると利用者に誤解される。
- CSVを表計算ソフトで開いたときに、タスク名やメモが数式として実行される。

## 実装結果

- `DataExportRepository` を追加し、Application Use Case、DTO、Tauri command、frontend gateway契約を接続した。
- `create_json_export` は `TaskTimer-export-YYYYMMDD-HHMMSS.json` を作成する。
- `create_csv_export` は `TaskTimer-export-YYYYMMDD-HHMMSS-csv/` にmanifestと複数CSVを作成する。
- CSVでメモ本文の改行、カンマ、ダブルクォートが壊れないこと、数式セルを安全化することをテストした。

## レビュー記録

- 指摘事項: UIからの保存先選択と注意表示は #89 で扱う。
- 破綻シナリオ: CSVファイルの一部作成で失敗した場合、中途半端なCSVフォルダを削除する。
- スケール懸念: 大量データを一度メモリへ読み込むため、5,000件以上の実測は #72 で確認する。
- セキュリティ懸念: JSON/CSVには個人データが含まれる。公開Issue、PR、Release artifactへ添付しない。
- テスト不足: OS権限拒否、ディスクフル、巨大メモのエクスポート時間は後続の実機/性能確認対象。
- 判断: フォローアップ付き承認。設定画面UIでは保存前の注意文と成功/失敗表示を実装する。

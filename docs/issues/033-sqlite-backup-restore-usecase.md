# 033: SQLiteバックアップ/復元Use Caseを実装する

GitHub Issue: #88

## 目的

TaskTimerの完全復元用バックアップを、アプリ内Use Caseとして安全に作成・復元できるようにする。

## スコープ

- SQLiteバックアップ作成Use Caseを追加する。
- SQLite復元Use Caseを追加する。
- `backup-manifest.json` を生成・検証する。
- `PRAGMA integrity_check` と必須テーブル確認を行う。
- 破損DB、バージョン不一致、復元失敗時の扱いを実装する。

## スコープ外

- クラウド同期。
- 自動バックアップ。
- 暗号化バックアップ。
- JSON/CSVエクスポート。

## 設計レビュー

### データモデル

`tasktimer.sqlite3` を完全復元の正とする。バックアップmanifestは復元判断用メタデータであり、アプリDBの正規テーブルにはしない。

manifestには以下を記録する。

- `format`: `tasktimer-sqlite-backup`
- `formatVersion`: バックアップ形式の互換性判断。
- `appVersion`: 作成元アプリのバージョン。
- `schemaVersion`: 復元先アプリが理解できるDBスキーマ世代。現在は `5`。
- `createdAt`: 作成日時。
- `platform`: 作成元OS。
- `databaseFile`: `tasktimer.sqlite3`
- `integrityCheck`: 作成時の `PRAGMA integrity_check` 結果。

### トランザクション境界

バックアップ作成では、書き込み中の単純ファイルコピーを避け、一貫したSQLiteスナップショットを作る。復元では一時DBで検証とマイグレーション可否確認を行い、成功後に現DBを退避して入れ替える。

実装ではSQLiteの `VACUUM INTO` を使う。理由は、既存の `rusqlite` 構成に追加の外部依存を増やさず、単純ファイルコピーより安全なスナップショットをSQLite側で作れるため。

復元境界:

1. `backup-manifest.json` を検証する。
2. バックアップDBを読み取り専用で開き、整合性と必須テーブルを検証する。
3. バックアップDBをアプリデータディレクトリの一時DBへコピーする。
4. 一時DB上で既存マイグレーションと初期設定seedを適用できるか確認する。
5. 現DB接続を閉じ、現DBを `tasktimer-before-restore-*.sqlite3` として退避する。
6. 一時DBを `tasktimer.sqlite3` へ入れ替える。
7. 新DBで接続を開き直す。失敗時は退避DBを戻す。

### セキュリティ

バックアップには個人データが含まれる。ファイル内容、タスク名、メモ本文、通知本文をログへ出さない。

Application Use Caseではローカルパス入力に対して、空文字、4096文字超過、NUL文字を拒否する。Tauriの新しいFS/ネットワーク権限は追加しない。

### 代替案

SQLite backup APIを `rusqlite` の追加featureで使う案もある。ページ単位の進捗制御をしやすい一方、現時点の要件では進捗UIがスコープ外で、`VACUUM INTO` で一貫したバックアップを作れるため採用しない。

### トレードオフ

- `VACUUM INTO` はバックアップ先DBを新規作成するため、同名バックアップフォルダが存在する場合は上書きせず失敗させる。
- 復元成功後も退避DBを残すため、利用者のディスク使用量は一時的に増える。復元失敗時の安全性を優先する。
- UIは後続Issueで追加するため、現時点ではTauri commandとgateway境界のみ提供する。

## 受け入れ条件

- Use Case、Repository/Infrastructure境界、Tauri commandが設計資料と一致している。
- バックアップ作成と復元のファイル入れ替え境界が明示されている。
- 破損DB、バージョン不一致、書き込み中バックアップのテストがある。
- ユーザー内容をログへ出していない。

## 危険ケース

- バックアップ作成中の書き込みで破損ファイルができる。
- 復元失敗時に既存DBまで失われる。
- 新しいアプリで作成されたDBを古いアプリへ復元して起動不能になる。

## 実装結果

- `SqliteBackupRepository` を追加し、Application Use Case、DTO、Tauri command、frontend gateway契約を接続した。
- `create_sqlite_backup` は `TaskTimer-backup-YYYYMMDD-HHMMSS/` を作成し、DBとmanifestを保存する。
- `restore_sqlite_backup` は破損DB、必須テーブル不足、将来 `schemaVersion` を拒否する。
- 外部接続が未コミット書き込み中でも、バックアップがコミット済みスナップショットだけを含むことをテストした。

## レビュー記録

- 指摘事項: UIからのファイル選択は後続Issueで扱う。Use Caseは任意パスを受け取るため、UI側でもユーザー選択済みディレクトリだけを渡す。
- 破綻シナリオ: 復元後のDB再接続に失敗した場合、退避DBを戻す。
- スケール懸念: 大きなDBではバックアップ作成中に時間がかかる。進捗表示とキャンセルは後続改善候補。
- セキュリティ懸念: バックアップファイルは個人データを含む。ログへユーザー内容を出さず、外部通信も追加しない。
- テスト不足: OSの権限拒否、ディスクフル、非常に大きいDBでの所要時間は実機確認または後続Issue候補。
- 判断: フォローアップ付き承認。UI追加時に保存先選択、注意文、復元前確認を実装する。

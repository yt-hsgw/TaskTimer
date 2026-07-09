# 変更履歴

このプロジェクトは、利用者に見える変更をこのファイルで記録します。

## Unreleased

### 追加

- macOS Developer ID署名とApple公証を前提にしたRelease workflow設定を追加。
- v0.1.0のRelease notes草案を追加。

### 注意

- 初回公開Releaseは、macOS署名・公証Secrets登録とmacOS/Windows実機確認後に公開します。
- Windowsコード署名、ストア配布、Linux配布、自動更新はv0.1.0の対象外です。
- Tauri経由のglib advisoryはIssue #22で追跡し、Linux artifactは配布しません。

## 0.1.0

### 追加

- Tauri + React + TypeScript + SQLiteのMVP実装。
- タスク、サブタスク、単一アクティブタイマー、週カレンダー、ローカル通知設定。
- 公開前チェック、リリース前チェック、GitHub運用資料。
- 外部利用者向けのGitHub運用方針。
- GitHub Releases向けのWindows/macOSリリースビルドワークフロー。
- `CONTRIBUTING.md`、`SUPPORT.md`、MIT License。

### 配布状態

- v0.1.0は公開判定待ちです。公開前に [Release notes草案](docs/releases/v0.1.0.md) とRelease Issue #20の手動確認結果を確認します。

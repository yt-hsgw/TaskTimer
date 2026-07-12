# 変更履歴

このプロジェクトは、利用者に見える変更をこのファイルで記録します。

## Unreleased

### 変更

- v0.1.0公開後のREADME、Release notes、次作業一覧を通常Release状態へ更新。
- Windowsコード署名方針をADR 0005として追加し、v0.1.xでは未署名Windows artifactを既知制限付きで配布する判断を記録。
- OS復帰または再フォーカス時にタスク、アクティブタイマー、期限通知を再同期する挙動を追加。
- カレンダーの週/日/月切替、Googleカレンダー型の時間軸表示、サブタスク親名表示を追加。
- 設定画面で通知全体をON/OFFできるように変更。

## 0.1.0

### 追加

- Tauri + React + TypeScript + SQLiteのMVP実装。
- タスク、サブタスク、単一アクティブタイマー、週カレンダー、ローカル通知設定。
- 公開前チェック、リリース前チェック、GitHub運用資料。
- 外部利用者向けのGitHub運用方針。
- GitHub Releases向けのWindows優先リリースビルドワークフロー。
- `CONTRIBUTING.md`、`SUPPORT.md`、MIT License。

### 配布状態

- v0.1.0はWindows先行の通常Releaseとして公開済みです。詳細は [Release notes](docs/releases/v0.1.0.md) と [GitHub Release](https://github.com/yt-hsgw/TaskTimer/releases/tag/app-v0.1.0) を確認してください。
- macOS版はApple署名・公証Secrets登録とGatekeeper確認後に提供します。
- Windowsコード署名、ストア配布、Linux配布、自動更新はv0.1.0の対象外です。
- Tauri経由のglib advisoryはIssue #22で追跡し、Linux artifactは配布しません。

# 変更履歴

このプロジェクトは、利用者に見える変更をこのファイルで記録します。

## Unreleased

### 変更

- v0.1.0公開後のREADME、Release notes、次作業一覧を通常Release状態へ更新。
- Windowsコード署名方針をADR 0005として追加し、v0.1.xでは未署名Windows artifactを既知制限付きで配布する判断を記録。
- OS復帰または再フォーカス時にタスク、アクティブタイマー、期限通知を再同期する挙動を追加。
- カレンダーの週/日/月切替、Googleカレンダー型の時間軸表示、サブタスク親名表示を追加。
- 設定画面で通知全体をON/OFFできるように変更。
- 設定画面で通知送信の失敗履歴と再試行結果を確認できるように変更。
- タスク詳細を参照中心の表示へ変更し、期限時刻、サブタスク展開、サブタスク詳細から親タスクへ戻る導線を追加。
- 今日ビュー、期限表示、メモプレビュー、ペイン内スクロールを現行READMEと設計資料へ反映。
- 左ペインでカスタムリストの作成、名称変更、削除ができるようにし、選択中リストでタスクを分類できるように変更。
- Issue整理として、通知全体ON/OFFの #55 を完了扱いにし、大量データ性能検証 #72 とローカルデータ退避方針 #73 を追加。
- Linux限定の`glib` advisoryについて、Windows/macOS配布物への非到達を確認し、Release対象ガード、週次監視、ADR 0007を追加。
- タイマー一時停止中の長時間経過・DB再接続テストと、GitHub-hosted Windows VMでの復帰回帰workflowを追加。

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
- Tauri経由のglib advisoryはLinux限定依存としてADR 0007でリスク受容し、Linux artifactは配布しません。

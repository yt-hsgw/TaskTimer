# 大量データPresentation計測レビュー

対象: GitHub #72 大量データで一覧とカレンダー表示を検証する。

## 指摘事項

- SQLite計測が高速でも、Reactが大量DOMを同期描画するとビュー切替が停止する可能性がある。
- 現行 `list_tasks` と `list_task_rows` は200件上限で、Presentationに追加読み込みがない。性能を守る代わりに201件目以降へ到達できない。
- ブラウザのTauri mockだけでは、SQLite、IPC、WebView2、OSウィンドウの時間を測れない。
- 実データを性能検証へ使うと、タスク名やメモがログ・artifactへ混入する。

## 判断

フォローアップ付き承認。

- Windows runnerのSQLite 5,000件計測に、Presentation 200件描画計測を追加する。
- UI計測は初期表示、今日、お気に入り、かんばん、週/日/月カレンダー、右詳細を対象とする。
- 200件上限によるデータ欠落は #131 に分離し、本Issueで上限だけを引き上げない。
- Windows実機のTauri/WebView2確認は、引き続きリリース前の手動ゲートとして残す。

## データ境界

- `smoke`: 50タスク、各4サブタスク、4リスト。
- `standard`: 一覧集計5,000件、描画200タスク、各4サブタスク、12リスト。
- カレンダー項目は要求された表示範囲内だけを合成する。
- 個人データ、SQLiteファイル、通知本文を使わない。

## 破綻シナリオ

- ビュー切替の完了条件が既に存在するDOMと一致し、実際の更新前に計測を終了する。
- かんばんの200カード描画でD&D初期化が閾値を超える。
- 月表示が表示範囲外の予定までDOM化する。
- CI端末差で単発の遅延が発生し、不安定な必須チェックになる。
- Chromeを終了できず、workflowがタイムアウトする。

## スケール

- Production commandの上限と同じ200件を描画し、現実の1ページ最大負荷を測る。
- 5,000件全描画は行わない。ページング後も1ページの上限を計測対象にする。
- 各操作は1回のwall-clock時間を記録し、閾値超過をWARNまたは失敗として扱う。

## セキュリティ

- 合成データだけを使う。
- 外部通信、分析SDK、新しいTauri capabilityを追加しない。
- ViteとChromeはloopbackだけを使い、計測終了時にプロセスと一時プロファイルを削除する。
- ユーザー入力をHTMLとして挿入しない。

## テスト計画

- `smoke` と `standard` の引数検証。
- 初期表示と主要ビューの完了DOMを確認する。
- 閾値超過時に `--fail-on-warning` で非ゼロ終了する。
- Windows workflowのsummaryと短期artifactへテキスト結果だけを保存する。
- 既存READMEスクリーンショット生成が共通Chrome helper抽出後も成功する。

## 検証結果

- Presentation smoke: 50タスク、200サブタスク、4リストでWARN 0。複数回の保守的な記録で最大1,007ms。
- Presentation standard: 一覧集計5,000件、描画200タスク、800サブタスク、12リストでWARN 0。複数回の保守的な記録で最大1,411ms。
- Windows runner Presentation smoke: WARN 0。最大3,078msで、Chrome起動、PowerShell引数、summary、artifact保存まで成功。
- SQLite standard: 5,000タスク、20,000サブタスク、50,000停止済み履歴でWARN 0。最大68ms。
- Rust: 全92テスト成功。Clippyは全ターゲット・全機能を警告エラー扱いで成功。
- Frontend: TypeScript型検査、Vite本番ビルド、READMEスクリーンショット生成に成功。
- Privacy: 実行時外部通信・ログ・リモートアセット・更新機能権限の静的監査に成功。

## 残余リスク

- Windows runnerのChrome計測は確認済みだが、標準5,000件集計・200件描画プロファイルは手動workflowで継続確認する。
- Windows実機のTauri IPC、WebView2、OSウィンドウ操作は未計測であり、#72の手動ゲートとして残る。
- 201件目以降への到達性は未解消であり、#131を完了するまで大量DBの全タスクをUIから操作できない。

## 代替案

Tauri WindowsアプリをCIで起動し、OS UI Automationだけで時間を測る案。

不採用理由: GitHub-hosted runnerではWebView2とデスクトップセッションの揺らぎが大きく、DOMの完了条件も取得しにくい。現段階ではSQLiteとPresentationを分離した再現可能な計測を先に置き、実機確認を補完として維持する。

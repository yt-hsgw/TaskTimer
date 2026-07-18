# 次の作業

## 現在の判断

v0.1.0はWindows先行の通常Releaseとして公開済みです。
Windows実機確認、Windows runnerでのインストール検証、実行時外部通信の静的監査は完了済みです。

当面はWindows利用を主軸にし、macOS配布はApple Developer ID署名とApple公証の準備ができるまで後回しにします。

GitHub上で継続追跡しているOpen Issue:

- #58: OSスリープ・復帰時のタイマーと通知の実機確認結果を記録する。
- #24: macOS署名と公証を設定する。

## 判断理由

- Windows実機確認が完了したため、v0.1.0は検証版ではなく通常Releaseとして扱える。
- Windowsコード署名はADR 0005に従い、v0.1.xでは未署名配布を既知制限付きで継続する。
- macOSはユーザー要望どおり後回しにし、未署名・未公証artifactを外部利用者向けに出さない。
- アプリ実行時の外部通信なし、自動更新なし、ローカルSQLite保存という公開運用方針は維持する。

## トレードオフ

- Windows先行Releaseにすることで外部利用者が試しやすくなる一方、Windows SmartScreenなどの未署名警告は残る。
- macOS配布を遅らせることでGatekeeper警告を避けられる一方、macOSユーザーはv0.1.0を公式artifactとして利用できない。
- 静的監査はCIで再現しやすい一方、実行時のネットワーク挙動を完全保証するものではないため、必要に応じて実機監視を併用する。

## 代替案

macOS署名・公証とWindowsコード署名導入が整うまでv0.1.0を非公開にする。

不採用理由:

- Windows実機確認とWindows runner検証は完了しており、Windows利用者へ先に価値を届けられる。
- コード署名未設定はRelease notesで既知制限として説明できる。
- macOS未署名artifactを出さなければ、Apple Gatekeeper警告を外部利用者に踏ませずに済む。

## 次に着手しやすい優先順

1. #58 OSスリープ・復帰時のタイマーと通知の実機確認結果を記録する。#123 のWindowsネイティブ将来通知PoCも、インストール済みアプリでアプリ完全終了中の発火可否とアンインストール後の予約通知残存可否をこの実機確認結果に含める。
2. #24 macOS署名と公証を設定する。現ラベルはP1だが、Windows優先運用ではApple署名・公証準備ができるまで後回しにする。

## 完了済み

- #131 タスク一覧・今日・お気に入り・タグ・かんばんのカーソルページングと追加読み込み。
- #22 glib advisoryのWindows/macOS非到達確認、Linux配布ガード、ADR 0007によるリスク受容。
- SQLite接続とマイグレーション実行器の初期実装。
- 週カレンダー、アクティブタイマー、通知表示設定のRepository境界。
- `CreateTask`、`CreateSubtask`、`StartTimer`、`StopActiveTimer` のApplication Use Case。
- タスク/サブタスク作成、タイマー開始/停止のSQLiteトランザクション実装。
- 単一アクティブタイマー制約と停止時 `elapsed_seconds` 確定のテスト。
- タスク一覧とタスク詳細の読み取りRepository。
- UIのモックデータ削除とSQLite由来表示への切り替え。
- タスク/サブタスク作成フォームとタイマー開始/停止ボタンのUse Case接続。
- 週カレンダーの日付範囲DBクエリ表示。
- かんばんのカスタム状態、列/カードのドラッグ&ドロップ、列ごとの完了セクション。
- カレンダーの予定期間モデル、週/日/月の期間表示、両端のドラッグ/キーボード調整。
- 未完了サブタスクがある親タスクの確認付き完了フロー。
- タスク削除時の子サブタスク、タイマー履歴、通知ルールのソフト削除。
- サブタスク削除時のタイマー履歴、通知ルールのソフト削除。
- タイマー開始中の対象削除時にアクティブタイマー検索から除外する処理。
- 通知表示モード設定の保存。
- タスク/サブタスク作成時の通知ルール作成。
- 期限到来通知のOS通知adapter送信、成功/失敗状態保存、再試行導線。
- 設定画面での通知全体ON/OFF。
- 通知全体ON/OFF Issue #55 の完了整理。
- 通知失敗履歴と再試行結果のRepository/UI初期実装。現在の通常設定画面では履歴を表示しない。
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
- v0.1.0 Release notes。
- glib advisoryの週次監視workflowとRelease artifact対象ガード。
- Release target検証スクリプト。
- macOS署名・公証用のTauri設定、Release workflow下準備、preflight。
- Windows優先Release workflowへの切り替え。
- Windows runnerでのインストーラー最低限検証workflow。
- 実行時外部通信・ログ出力の静的監査。
- v0.1.0 Windows通常Release公開。
- v0.1.0 Windows実機確認。
- Release Issue #20、Windows実機確認Issue #48、runtime privacy audit Issue #49の完了。
- Windowsコード署名方針ADR 0005。
- OSスリープ/復帰相当の再同期とタイマー/通知の自動テスト。
- 週/日/月カレンダー、時間軸表示、サブタスク親名表示。
- 今日ビュー、タスク行の期限表示、メモプレビュー、ペイン内スクロール。
- カスタムリストの作成、名称変更、削除、選択中リストへのタスク作成。
- 大量データ検証DB生成ツールと計測手順。
- GitHub-hosted Windows runnerでの大量データRead Model計測workflow。
- GitHub-hosted Windows runnerでの標準プロファイル大量データRead Model計測。
- #72 Windows runnerでの標準プロファイル大量データRead Model・Presentation描画計測と性能検証完了。
- ローカルデータのバックアップとエクスポート方針。
- #88 SQLiteバックアップ/復元Use Case。
- #87 JSON/CSVエクスポートUse Case。
- #89 バックアップ/復元/エクスポートUIの初期実装。現在の通常設定画面ではJSON/CSVエクスポートだけを表示する。
- #94 Rust静的解析CIの実行時間短縮。
- #52 タスクのアーカイブ操作Use Case、アーカイブ一覧Read Model、通常一覧/カレンダー/通知dispatch除外。
- #57 UI設定の永続化範囲定義、Get/Update Use Case、起動時復元、変更時保存。
- #82 カレンダーからのタスク追加。
- #83 カレンダー上での期限調整。
- #81 かんばん形式の画面追加と `UpdateTaskStatus` Use Case。
- #84 カレンダー上のリスト色反映。
- #80 親タスクへのタグ付与、タグ別ビュー、タグのJSON/CSVエクスポート。
- #107 ポモドーロタイマーのドメインモデル、Repository境界、設定/開始Use Case、SQLiteマイグレーション、JSON/CSVエクスポート、作業完了/休憩開始/完了/スキップ/キャンセルのフェーズ遷移スライス、設定画面での既定値編集UI、右詳細ペインでのポモドーロ操作UI、期限到達同期、作業/休憩終了通知、自動開始設定の実行処理、大量データ計測へのactive pomodoro lookup追加。
- #51 OSへの将来時刻通知スケジューリング方式設計。アプリ起動中スケジューラを第1段階、Windows/macOSネイティブ永続登録検証を第2段階として分割した。
- #116 アプリ起動中の将来時刻通知スケジューラ。次回通知予定Read Use Case、Tauri command、React単一timer、通知OFF/重複防止テストを追加した。
- #117 起動・復帰・設定変更時の通知再同期。`sync_notifications` を入口にし、期限到来dispatch後に次回通知予定を再予約する境界とテストを追加した。
- #115 通知OS登録状態のRepository境界とDB状態。`notification_os_registrations`、Repository/Use Case境界、既存通知ルールbackfill、タスク更新/削除時のOS登録状態同期を追加した。
- #118 Windows/macOSネイティブ将来通知adapterの実現性検証。Tauri plugin 2.3.3 のdesktop `show()` は永続予約として扱えないため本実装せず、Windows先行PoC #123 に分割した。macOSは署名・公証準備まで後回しにする。
- #123 Windowsネイティブ将来通知adapterのPoC。Rust Infrastructure内にWindows限定 `ScheduledToastNotification` adapter、Application Use Case、Tauri command、起動時同期呼び出しを追加した。公開保証としての採用は、Windows 11インストール済みアプリで登録、変更、解除、通知全体OFF、`generic` 表示、アプリ完全終了中の発火、アンインストール後の予約通知残存可否を確認してから判断する。
- Issue 050 作業画面の操作配置と表示密度整理。詳細オーバーレイ、詳細内の所属リスト/リスト色/タグ管理、カレンダーのダブルクリック追加、かんばん完了チェック、通知表示カード、JSON/CSV限定設定を反映した。

## 実務運用時に継続確認すること

1. Windows未署名artifactのOS警告をRelease notesへ既知制限として維持する。
2. 不具合報告には、個人のタスク名、メモ本文、通知本文、SQLiteファイル、秘密情報を貼らない。
3. 新しい外部通信、自動更新、リモートアセットを追加する場合は、ADRとRelease notesを更新して明示承認を取る。
4. macOS artifactを配布する場合は、macOS署名・公証用GitHub Secretsを登録する。
5. macOS artifactを配布する場合は、macOS DMGを実機で開き、Gatekeeper警告が解消されることを確認する。

## 危険ケース

- Windows通常Releaseなのに、READMEやRelease notesが公開待ちまたはpre-releaseのまま残る。
- Windows runnerのインストール検証成功を、実機の通知、GUI、SmartScreen確認完了と誤認する。
- macOS artifactを含める時に、macOS署名・公証Secrets未登録のままRelease workflowを実行する。
- `npm run check:macos-signing` の失敗を無視してmacOS artifactを公開する。
- 古いcommitで生成したRelease artifactを公開し、Release notesや手動確認結果と実artifactが食い違う。
- Gatekeeper警告が残るDMGを外部利用者向けに公開する。
- Windows未署名警告をRelease notesへ書かず、利用者がインストール可否を判断できない。
- dismiss済みglib advisoryの判断をRelease notesとADRから消し、Linux配布対象外の理由が伝わらない。
- Issue、PR、Release notesへApple証明書、Apple ID、App用パスワード、Team ID、Windows署名用の証明書、秘密鍵、Azure認証情報、ローカルDB、個人タスク内容を貼ってしまう。

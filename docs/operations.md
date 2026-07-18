# 運用方針

## GitHub管理

GitHubで管理するもの:

- Issue。
- Pull Request。
- Discussions。
- 設計レビューの議論。
- Release tag。
- Release notes。

GitHubをアプリ実行時データの保存先には使わない。

## パブリック公開

公開前に [パブリック公開前チェック](public-readiness.md) を確認する。

公開してよいもの:

- ソースコード。
- 設計資料。
- IssueとPull Requestの議論。
- CI結果。
- Release notes。

公開してはいけないもの:

- 秘密情報。
- ローカルSQLiteデータベース。
- SQLiteバックアップ、JSONエクスポート、CSVエクスポート。
- タスク名、メモ本文、通知本文などの実データ。
- 個人環境の絶対パス。
- 署名用証明書、秘密鍵、APIキー。

ライセンス方針:

- MIT Licenseを採用する。
- ライセンス判断は [ADR 0004](adr/0004-public-distribution-license.md) に記録する。
- 外部からの貢献はMIT Licenseで提供されるものとして扱う。

依存関係運用:

- Dependabotでnpm、Cargo、GitHub Actionsの更新を追跡する。
- DependabotやActionsの通信は開発・運用時の通信であり、アプリ実行時の外部通信ではない。
- 依存更新PRでは、Tauri権限、外部通信、ログ出力、通知プライバシーへの影響を確認する。
- Tauri経由のLinux限定 `glib` advisoryは [ADR 0007](adr/0007-glib-linux-target-risk-acceptance.md) に従い、Windows/macOS配布物では未使用としてリスク受容する。
- `依存関係アラート監視` ワークフローで週次再評価し、失敗した場合は上流依存が更新可能になったか、配布対象OSの境界が変わった合図として依存更新または設計再審査へ進む。

## GitHub Actions

`リポジトリチェック` ワークフローをPRとブランチpushで実行する。必要に応じてGitHub Actions画面から手動実行する。

確認するもの:

- 必須設計ファイル。
- SQLiteスキーマと初期マイグレーション。
- Rust format、test、clippy。
- TypeScript/Vite build。
- npm audit。
- `.env`、DB、鍵、証明書、ログの誤コミット。
- 個人環境パスとメールアドレスの誤コミット。
- 空白エラー。

注意:

- Actions内の依存取得やGitHub通信は開発・運用時の通信であり、アプリ実行時の外部通信ではない。
- Rust CIは `Swatinem/rust-cache` でCargo registry、Cargo git、`src-tauri/target` をキャッシュする。キャッシュ導入後も `cargo test` と `cargo clippy --all-targets -- -D warnings` は維持する。
- Rust cacheの効果はGitHub Actionsの実行時間で確認する。`Cargo.lock`、Rust toolchain、OSが変わる場合はキャッシュが分かれる前提で扱う。
- OS固有の通知権限、インストーラー、署名警告はCIだけでは保証しない。リリース前チェックリストで手動確認する。

`Windows復帰回帰検証` ワークフローは、関連するPull Requestと手動実行でWindows runner上のRustテストを実行する。

- 開始中・一時停止中タイマーのDB再接続とwall-clock差分を検証する。
- 復帰相当の通知同期と重複送信防止を検証する。
- 実際のOS電源スリープ、WebView2のフォーカス復帰、OS通知表示は保証しないため、Release前手動確認を省略しない。
- workflow権限は `contents: read` に限定する。

`依存関係アラート監視` ワークフローは、週1回と手動実行で `glib` advisoryの上流制約を再評価する。

Dependency advisory workflowの権限:

- `contents: read`。Cargo resolver確認だけに必要な最小権限として扱う。

Dependency advisory workflowの制約:

- アプリ実行時の通信やTauri権限は追加しない。
- Issueコメントの自動投稿は行わない。
- 修正可能になった場合はworkflowを失敗させ、依存更新PR作成を促す。
- Windows/macOSの依存グラフへ`glib`が入った場合、またはRelease matrixへLinux・未知ターゲットが追加された場合は失敗させる。

## ローカルデータ保護

TaskTimerの完全復元用バックアップはSQLiteバックアップを正とする。JSON/CSVエクスポートは閲覧、監査、他ツール移行の補助形式であり、完全復元用には使わない。

運用ルール:

- バックアップ/エクスポートファイルは個人データとして扱う。
- Issue、PR、Discussions、Release artifactへ添付しない。
- GitHubでの不具合調査では、実DBではなく再現手順または合成データを使う。
- バックアップ/復元/エクスポートの設計は [ローカルデータのバックアップとエクスポート方針](data-backup-export.md) と [ADR 0006](adr/0006-local-backup-export-policy.md) に従う。
- アプリ内実装では、DB書き込み中の単純ファイルコピーを避け、一貫したスナップショットを作る。

`リリースビルド` ワークフローは、`app-v*` タグまたは手動実行でWindows向けartifactをビルドし、Draft Releaseへ添付する。macOS artifactは手動実行で `include_macos` を有効にした場合だけ作成する。

`Windowsインストーラー検証` ワークフローは、手動実行で公開済みReleaseまたはpre-releaseのWindows artifactをGitHub-hosted Windows runnerへ取得し、NSISのサイレントインストールとサイレントアンインストールを検証する。

`大量データ性能検証` ワークフローは、PRではスモークプロファイル、手動実行では標準プロファイルの検証DBをGitHub-hosted Windows runnerで生成し、Read Model相当のSQLiteクエリ時間を計測する。

Release workflowの権限:

- workflow全体は `contents: read` を基本権限として扱う。
- `build-release` jobだけ `contents: write` を持つ。Releaseとartifact作成に必要な最小権限として扱う。
- macOS署名・公証用SecretsはRepository Secretsとして扱い、workflowログ、Issue、PR、Release notesには出さない。
- Windowsコード署名を導入する場合のSecrets、workflow、確認手順は [ADR 0005](adr/0005-windows-code-signing-policy.md) に従って別Issueで設計する。証明書、秘密鍵、証明書パスワード、Azure認証情報はworkflowログ、Issue、PR、Release notesには出さない。

Release workflowの制約:

- Draft Releaseとして作成する。
- 自動更新artifactは作成しない。
- v0.1.0の主配布対象はWindowsとする。
- macOS artifactはApple Developer Programと署名・公証Secretsの準備が完了するまで後回しにする。
- Windowsコード署名はv0.1.xでは導入せず、未署名配布を既知制限付きで継続する。
- macOS artifactを作成する場合はDeveloper ID署名とApple公証を行う。
- macOS署名・公証Secretsが未設定の場合、`preflight-macos` jobで失敗させ、macOS込みmatrix buildへ進めない。
- macOS job内でもSecrets検証を行い、matrix実行時の防御層として扱う。
- 公開前に `docs/release-checklist.md` の手動確認を完了する。

Windows installer smoke workflowの権限:

- `contents: read`。公開済みRelease assetの読み取りに必要な最小権限として扱う。

Windows installer smoke workflowの制約:

- 実機での通知、GUI操作、SmartScreen表示確認の代替にしない。
- 成功した場合も、Release notesにはWindows実機確認または未確認状態を明示する。
- Release asset取得には `GITHUB_TOKEN` を使い、追加Secretは使わない。
- Draft Releaseを `GITHUB_TOKEN` で取得できない場合は、通常Releaseとして公開せず、検証版として公開するか別手段でartifactを取得し、確認結果をRelease notesへ追記する。

Large dataset performance workflowの権限:

- `contents: read`。ソース取得と検証binの実行に必要な最小権限として扱う。

Large dataset performance workflowの制約:

- Windows runner上のRead Model計測であり、Windows実機のGUI操作、通知権限、Tauri IPC、SmartScreen確認の代替にしない。
- PRではスモークデータだけを自動実行し、標準データは `workflow_dispatch` で明示実行する。
- 生成DBファイルはartifactへ保存しない。計測ログだけを短期artifactとstep summaryへ残す。
- 追加Secret、新しいTauri権限、アプリ実行時外部通信は使わない。

## ブランチ運用

推奨:

- `main`: リリース可能な状態。
- `feature/<short-name>`: 機能開発。
- `fix/<short-name>`: 不具合修正。
- `docs/<short-name>`: 設計・資料更新。
- Codex作業ブランチ: `codex/<short-name>`。

## Pull Request要件

各PRには以下を含める。

- 関連Issueまたは設計理由。
- ユーザーに見える変更の概要。
- データモデルへの影響。
- トランザクション境界への影響。
- セキュリティ影響。
- テスト証跡。
- デスクトップ挙動が変わる場合の手動確認。

## リリース要件

リリース前に確認する。

- `docs/public-readiness.md` を確認する。
- `docs/public-operations.md` を確認する。
- `docs/release-checklist.md` を確認する。
- `docs/review/checklist.md` を確認する。
- 自動テストを実行する。
- Windowsで手動デスクトップ確認を行う。
- macOS artifactを配布する場合はmacOSでも手動デスクトップ確認を行う。
- アプリ実行時の外部通信がないことを確認する。
- ユーザー内容をログへ出していないことを確認する。
- ローカル通知挙動と通知全体ON/OFFを確認する。
- OSスリープ/復帰後のタイマー復元、経過時間、通知重複なしを確認する。
- Draft Releaseのartifact名、Release notes、既知制限を確認する。
- macOS artifactを配布する場合は、macOS DMGの署名・公証・Gatekeeper確認を完了する。

## 配布形式

MVPの配布形式:

- Windows: `nsis`。
- macOS: `dmg`。Apple署名・公証準備が完了したReleaseでのみ提供する。

理由:

- 現在のTauri設定 `src-tauri/tauri.conf.json` と一致する。
- WindowsではNSISによりインストール/アンインストール導線を用意できる。
- macOSでは一般的なドラッグ&ドロップ配布にできる。
- 自動更新artifactはMVPでは作成しないため、リモート更新エンドポイントを必要としない。

トレードオフ:

- v0.1.0ではWindows配布を先行し、macOS利用者向けの正式artifact提供は遅れる。
- macOS署名・公証にはApple Developer ProgramとSecrets運用が必要になる。
- Windowsコード署名はv0.1.xでは未導入のため、SmartScreenまたは組織ポリシーの警告が出る可能性がある。
- `msi` やストア配布よりも企業端末での一括配布には弱い。
- Windowsコード署名の判断は [ADR 0005](adr/0005-windows-code-signing-policy.md) に従う。導入する場合は別IssueでSecrets、workflow、確認手順を設計する。

代替案:

- Windowsを `msi` にする。企業配布には向くが、MVPでは運用準備が増える。
- GitHub Releasesで自動更新artifactを作る。利便性は上がるが、アプリ実行時の外部通信禁止方針と衝突するためMVPでは採用しない。

## ローカル通知手動確認

Windowsでは必ず確認する。macOS artifactを配布する場合はmacOSでも確認する。

1. 通知全体をOFFにする。
2. 今日を期限にしたタスクを作成し、OS通知が送信されないことを確認する。
3. 通知全体をONに戻し、設定画面から再試行できることを確認する。
4. 通知表示タイプを `タイトルのみ` にする。
5. 今日を期限にしたタスクを作成する。
6. 設定画面の通知処理結果が成功になることを確認する。
7. OS通知にメモ本文が含まれないことを確認する。
8. 通知表示タイプを `汎用メッセージ` にする。
9. 今日を期限にした別タスクを作成する。
10. OS通知にタスク/サブタスクタイトルが含まれないことを確認する。
11. OS通知権限が拒否されている状態で、設定画面に失敗が表示され、再試行できることを確認する。

注意:

- Tauri notification pluginの仕様上、Windowsでは開発実行時とインストール済みアプリで通知の表示名やアイコンが異なる場合がある。
- MVPではアプリ起動中または再読み込み時に期限到来済み通知を送信する。OSへの将来時刻スケジューリングは後続改善で扱う。
- OS復帰またはウィンドウ再フォーカス時は、TaskTimerが状態を再同期し、登録済み通知を重複送信しないことを確認する。

## 設定方針

実行時設定はローカルかつ明示的に管理する。

許可:

- SQLiteまたはアプリ設定ファイルに保存するユーザー設定。
- 通知設定。
- ウィンドウサイズやレイアウト設定。

禁止:

- APIキー。
- リモートエンドポイントURL。
- 分析識別子。
- ネットワークアクセスを有効化する隠しFeature Flag。

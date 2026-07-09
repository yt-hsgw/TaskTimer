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
- Tauri経由の `glib` advisoryは、`依存関係アラート監視` ワークフローで週次再評価する。
- 監視ワークフローが失敗した場合は、上流依存が更新可能になった合図としてIssue #22から依存更新PRへ進む。

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
- OS固有の通知権限、インストーラー、署名警告はCIだけでは保証しない。リリース前チェックリストで手動確認する。

`依存関係アラート監視` ワークフローは、週1回と手動実行で `glib` advisoryの上流制約を再評価する。

Dependency advisory workflowの権限:

- `contents: read`。Cargo resolver確認だけに必要な最小権限として扱う。

Dependency advisory workflowの制約:

- アプリ実行時の通信やTauri権限は追加しない。
- Issueコメントの自動投稿は行わない。
- 修正可能になった場合はworkflowを失敗させ、依存更新PR作成を促す。

`リリースビルド` ワークフローは、`app-v*` タグまたは手動実行でWindows/macOS向けartifactをビルドし、Draft Releaseへ添付する。

Release workflowの権限:

- `contents: write`。Releaseとartifact作成に必要な最小権限として扱う。
- macOS署名・公証用SecretsはRepository Secretsとして扱い、workflowログ、Issue、PR、Release notesには出さない。

Release workflowの制約:

- Draft Releaseとして作成する。
- 自動更新artifactは作成しない。
- macOS artifactはDeveloper ID署名とApple公証を行う。
- macOS署名・公証Secretsが未設定の場合、macOSジョブは失敗させる。
- 公開前に `docs/release-checklist.md` の手動確認を完了する。

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
- Windows/macOSで手動デスクトップ確認を行う。
- アプリ実行時の外部通信がないことを確認する。
- ユーザー内容をログへ出していないことを確認する。
- ローカル通知挙動を確認する。
- Draft Releaseのartifact名、Release notes、既知制限を確認する。
- macOS DMGの署名・公証・Gatekeeper確認を完了する。

## 配布形式

MVPの配布形式:

- macOS: `dmg`。
- Windows: `nsis`。

理由:

- 現在のTauri設定 `src-tauri/tauri.conf.json` と一致する。
- macOSでは一般的なドラッグ&ドロップ配布にできる。
- WindowsではNSISによりインストール/アンインストール導線を用意できる。
- 自動更新artifactはMVPでは作成しないため、リモート更新エンドポイントを必要としない。

トレードオフ:

- macOS署名・公証にはApple Developer ProgramとSecrets運用が必要になる。
- Windowsコード署名は未設定のため、SmartScreenなどのOS警告が出る可能性がある。
- `msi` やストア配布よりも企業端末での一括配布には弱い。
- 公開配布前には署名とインストール手順を別ADRで再検討する。

代替案:

- Windowsを `msi` にする。企業配布には向くが、MVPでは運用準備が増える。
- GitHub Releasesで自動更新artifactを作る。利便性は上がるが、アプリ実行時の外部通信禁止方針と衝突するためMVPでは採用しない。

## ローカル通知手動確認

macOS/Windowsの両方で確認する。

1. 通知表示タイプを `タイトルのみ` にする。
2. 今日の日付を開始日または終了日にしたタスクを作成する。
3. 設定画面の通知処理結果が成功になることを確認する。
4. OS通知にメモ本文が含まれないことを確認する。
5. 通知表示タイプを `汎用メッセージ` にする。
6. 今日の日付を開始日または終了日にした別タスクを作成する。
7. OS通知にタスク/サブタスクタイトルが含まれないことを確認する。
8. OS通知権限が拒否されている状態で、設定画面に失敗が表示され、再試行できることを確認する。

注意:

- Tauri notification pluginの仕様上、Windowsでは開発実行時とインストール済みアプリで通知の表示名やアイコンが異なる場合がある。
- MVPではアプリ起動中または再読み込み時に期限到来済み通知を送信する。OSへの将来時刻スケジューリングは後続改善で扱う。

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

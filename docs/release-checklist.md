# リリース前チェックリスト

## 目的

Windows/macOS向けにTaskTimerを配布する前に、実務運用で必要な品質、セキュリティ、オフライン方針を確認する。

このチェックリストは、GitHub Releaseを作成する前の手動ゲートとして使う。GitHub Actionsは基本的なビルドとテストを確認するが、OS通知やインストーラー挙動はOSごとの手動確認を必須とする。

## リリース単位

- バージョンは `src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`package.json` の整合を確認する。
- 配布形式はTauri設定に合わせる。
- macOS: `dmg`。
- Windows: `nsis`。
- GitHub Release tagは `app-vX.Y.Z` 形式にする。
- Release tagは意図したリリース対象commitを指している必要がある。
- GitHub Actionsの `リリースビルド` でDraft Releaseを作成する。
- `リリースビルド` は `preflight-release` 成功後にWindows/macOSのmatrix buildへ進む。
- 自動更新用artifactはMVPでは作成しない。`createUpdaterArtifacts` は `false` を維持する。
- macOS artifactはDeveloper ID署名とApple公証を必須にする。
- Windowsコード署名は未設定のため、Release notesに既知制限として記載する。

## 事前条件

- `main` がリリース対象のコミットを指している。
- 関連IssueとPRがGitHub上で追跡できる。
- 未解決の `priority: P0` または `priority: P1` Issueがない。
- `docs/public-readiness.md` を確認済みである。
- `docs/public-operations.md` を確認済みである。
- `docs/releases/<version>.md` のRelease notes草案を確認済みである。
- リリースノートに既知制限、手動確認結果、外部通信方針を記載できる。
- 未解決のDependabot alertがある場合は、影響範囲、配布対象、追跡IssueをRelease notesに記載できる。
- macOS署名・公証用GitHub Secretsを登録済みである。
- Secrets値をIssue、PR、Release notes、ログに貼っていない。

Release tagとtarget commitを確認する。

```bash
git fetch origin main --tags
npm run check:release-target -- <version> origin/main
```

既存Draft Releaseが古いcommitのartifactを持っている場合は、Draft Releaseを公開せず、Release notesと手動確認結果を引き継いだうえでDraft Releaseとtagを作り直す。

## macOS署名・公証Secrets

GitHub Repository Secretsに以下を登録する。

- `APPLE_CERTIFICATE`: Developer ID Application証明書を `.p12` でexportし、base64化した値。
- `APPLE_CERTIFICATE_PASSWORD`: `.p12` export時のパスワード。
- `APPLE_SIGNING_IDENTITY`: `security find-identity -v -p codesigning` で確認した署名ID。
- `APPLE_ID`: 公証に使うApple ID。
- `APPLE_PASSWORD`: Apple IDのApp用パスワード。
- `APPLE_TEAM_ID`: Apple Developer Team ID。

Secrets値はローカルファイル、Issue、PR、Release notesに保存しない。

Release workflow実行前にpreflightを実行する。

```bash
npm run check:macos-signing
```

このコマンドはSecrets値を読み取らず、登録済みSecret名とTauri設定、Entitlements、macOS検証ツールの存在だけを確認する。失敗した場合はDraft Releaseを公開しない。

## 自動チェック

GitHub Actionsで以下が成功していることを確認する。

- Release前提確認job。
- 必須設計ファイルの存在確認。
- SQLiteスキーマと初期マイグレーションの検証。
- Rust format check。
- Rust unit/integration test。
- Rust clippy。
- npm audit。
- TypeScript/Vite build。
- macOS署名・公証Secrets検証。
- `.env` と `.env.*` の誤コミット検出。
- DB、鍵、証明書、ログ、個人環境パス、メールアドレスの誤コミット検出。
- `git diff --check`。

ローカルで確認する場合:

```bash
npm ci
sqlite3 :memory: ".read docs/database-schema.sql"
sqlite3 :memory: ".read src-tauri/migrations/0001_initial.sql"
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run build
git diff --check
```

## 手動デスクトップ確認

macOSとWindowsの両方で確認する。

- インターネット接続なしでアプリが起動する。
- タスクを作成、完了、削除できる。
- サブタスクを作成、完了、削除できる。
- タスクタイマーを開始、停止できる。
- サブタスクタイマーを開始、停止できる。
- 同時に開始できるタイマーが1件だけである。
- タイマー開始中にアプリを再起動しても、アクティブタイマーが復元される。
- 週カレンダーで開始日と終了日のあるタスク/サブタスクが表示される。
- 通知表示タイプ `タイトルのみ` でメモ本文が通知に出ない。
- 通知表示タイプ `汎用メッセージ` でタスク/サブタスクタイトルが通知に出ない。
- 通知権限拒否時に設定画面から失敗と再試行導線が分かる。
- OSスリープ/復帰後にタイマーと通知状態が破綻しない。
- macOS DMGを開いたときに「Appleが検証できません」警告が出ない。
- macOSで `spctl --assess --type execute --verbose /Applications/TaskTimer.app` が成功する。
- macOSで `xcrun stapler validate /Applications/TaskTimer.app` が成功する。

## セキュリティ確認

- アプリ実行時の外部通信を追加していない。
- リモートフォント、リモート画像、分析、クラッシュアップロード、自動更新通信を追加していない。
- 新しいTauri権限を追加した場合は、PR本文と設計資料に理由がある。
- タスク名、サブタスク名、メモ本文、通知本文をログに出していない。
- メモ本文をHTMLとして描画していない。
- 秘密情報、DBファイル、個人データをRelease artifactやIssue/PRに添付していない。
- Apple証明書、Apple ID、App用パスワード、Team IDをログやGitHub本文へ出していない。
- Git履歴の著者情報を公開してよいか確認済みである。

## リリース作成手順

1. `main` のGitHub Actionsが成功していることを確認する。
2. macOS/Windowsの手動確認結果をリリースIssueへ記録する。
3. `app-vX.Y.Z` タグを `main` の対象コミットへ作成してpushする。またはGitHub Actionsから `リリースビルド` を手動実行する。
4. `リリースビルド` の `preflight-release` が成功し、Draft ReleaseへWindows/macOS artifactを添付することを確認する。
5. `npm run check:release-target -- <version> origin/main` でtagと公開対象commitが一致することを確認する。
6. `npm run check:macos-signing` でmacOS署名・公証preflightが成功することを確認する。
7. macOSジョブで署名・公証が成功していることを確認する。
8. macOSでは生成された `dmg`、Windowsでは生成された `nsis` artifactを手動でインストール確認する。
9. `docs/releases/<version>.md` の草案をもとに、Release notesへ変更点、既知制限、手動確認結果、外部通信なしの方針を記載する。
10. Windowsコード署名未設定によるOS警告の可能性を既知制限に記載する。
11. 未解決のDependabot alertがある場合は、影響範囲、配布対象、追跡Issueを既知制限に記載する。
12. Draft Releaseを公開する。

ローカルでartifactを作る場合:

```bash
npm run tauri:build
```

## ロールバック

- 重大な不具合が見つかった場合、該当Releaseをpre-releaseまたはdraftへ戻す。
- GitHub Issueに影響範囲、再現手順、回避策を記録する。
- DBマイグレーションを含む不具合では、データ復旧方針を確認するまで修正版を公開しない。

## 破綻シナリオ

- CIは通るが、WindowsまたはmacOS固有の通知権限で通知が届かない。
- インストール済みアプリでは通知表示名やアイコンが開発時と異なる。
- macOS署名・公証Secretsが未設定でRelease workflowが失敗する。
- macOS署名・公証preflightの失敗を無視してDraft Releaseを公開する。
- macOS署名・公証Secretsが未設定のままmatrix buildが始まり、Windows artifactだけがDraft Releaseへ残る。
- 公証が失敗したDMGを公開してしまい、Gatekeeper警告により業務利用者へ配布できない。
- Windows未署名artifactがOSセキュリティ警告により業務利用者へ配布しにくい。
- リリースノートに既知制限がなく、利用者が通知やタイマー復元の仕様を誤解する。
- 依存関係アラートの影響範囲をRelease notesに書かず、利用者が配布対象OSとリスクを判断できない。
- 古いcommitで生成したDraft Release artifactを公開し、Release notesや手動確認結果と実artifactが食い違う。
- `.env.*` や個人DBファイルをartifactまたはIssueへ添付してしまう。

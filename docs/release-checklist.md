# リリース前チェックリスト

## 目的

Windows/macOS向けにTaskTimerを配布する前に、実務運用で必要な品質、セキュリティ、オフライン方針を確認する。

このチェックリストは、GitHub Releaseを作成する前の手動ゲートとして使う。GitHub Actionsは基本的なビルドとテストを確認するが、OS通知やインストーラー挙動はOSごとの手動確認を必須とする。

## リリース単位

- バージョンは `src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`package.json` の整合を確認する。
- 配布形式はTauri設定に合わせる。
- macOS: `dmg`。
- Windows: `nsis`。
- 自動更新用artifactはMVPでは作成しない。`createUpdaterArtifacts` は `false` を維持する。
- 署名は別ADRで承認されるまで必須化しない。公開配布前には署名方針を再確認する。

## 事前条件

- `main` がリリース対象のコミットを指している。
- 関連IssueとPRがGitHub上で追跡できる。
- 未解決の `priority: P0` または `priority: P1` Issueがない。
- `docs/public-readiness.md` を確認済みである。
- リリースノートに既知制限、手動確認結果、外部通信方針を記載できる。

## 自動チェック

GitHub Actionsで以下が成功していることを確認する。

- 必須設計ファイルの存在確認。
- SQLiteスキーマと初期マイグレーションの検証。
- Rust format check。
- Rust unit/integration test。
- Rust clippy。
- npm audit。
- TypeScript/Vite build。
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

## セキュリティ確認

- アプリ実行時の外部通信を追加していない。
- リモートフォント、リモート画像、分析、クラッシュアップロード、自動更新通信を追加していない。
- 新しいTauri権限を追加した場合は、PR本文と設計資料に理由がある。
- タスク名、サブタスク名、メモ本文、通知本文をログに出していない。
- メモ本文をHTMLとして描画していない。
- 秘密情報、DBファイル、個人データをRelease artifactやIssue/PRに添付していない。
- Git履歴の著者情報を公開してよいか確認済みである。

## リリース作成手順

1. `main` のGitHub Actionsが成功していることを確認する。
2. macOS/Windowsの手動確認結果をリリースIssueへ記録する。
3. `npm run tauri:build` を対象OS上で実行する。
4. macOSでは生成された `dmg`、Windowsでは生成された `nsis` artifactを手動でインストール確認する。
5. GitHub Release draftを作成する。
6. Release notesに変更点、既知制限、手動確認結果、外部通信なしの方針を記載する。
7. Release tagを作成し、artifactを添付する。

## ロールバック

- 重大な不具合が見つかった場合、該当Releaseをpre-releaseまたはdraftへ戻す。
- GitHub Issueに影響範囲、再現手順、回避策を記録する。
- DBマイグレーションを含む不具合では、データ復旧方針を確認するまで修正版を公開しない。

## 破綻シナリオ

- CIは通るが、WindowsまたはmacOS固有の通知権限で通知が届かない。
- インストール済みアプリでは通知表示名やアイコンが開発時と異なる。
- 署名なしartifactがOSセキュリティ警告により業務利用者へ配布できない。
- リリースノートに既知制限がなく、利用者が通知やタイマー復元の仕様を誤解する。
- `.env.*` や個人DBファイルをartifactまたはIssueへ添付してしまう。

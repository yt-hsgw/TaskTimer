# コントリビュート方針

TaskTimerへのIssue、Discussion、Pull Requestを歓迎します。

## 先に確認すること

- 既存のIssueとDiscussionsを確認してください。
- 機能追加や挙動変更は、実装前に目的、データモデル影響、トランザクション境界、セキュリティ影響をIssueで説明してください。
- 実データを含むSQLiteファイル、タスク名、メモ本文、通知本文、秘密情報、個人的なスクリーンショットは投稿しないでください。

## 開発手順

```bash
npm ci
npm run tauri:dev
```

よく使う確認コマンド:

```bash
npm run build
sqlite3 :memory: ".read docs/database-schema.sql"
sqlite3 :memory: ".read src-tauri/migrations/0001_initial.sql"
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
git diff --check
```

## Pull Request要件

PRには以下を含めてください。

- 関連Issueまたは設計理由。
- ユーザーに見える変更の概要。
- データモデルへの影響。
- トランザクション境界への影響。
- セキュリティ影響。
- テスト証跡。
- デスクトップ挙動が変わる場合の手動確認結果。

## 設計原則

- UIではなくデータモデルから設計する。
- Clean Architectureを基本にする。
- Application Use Caseをトランザクション境界にする。
- InfrastructureにRepository実装と副作用アダプターを置く。
- OS通知はDBコミット後の副作用として扱う。
- アプリ実行時の外部通信は追加しない。

## ライセンス

このリポジトリへの貢献は、リポジトリと同じMIT Licenseで提供されるものとして扱います。

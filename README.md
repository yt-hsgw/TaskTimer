# TaskTimer

Windows/macOS向けの、オフライン前提TODO・タイマー管理デスクトップアプリです。

## プロダクト範囲

TaskTimerは、タスク、サブタスク、予定日、ローカル通知、タイマー履歴を端末内だけで管理します。アプリ実行時の外部通信は行いません。

MVPの決定事項:

- Windows/macOS向けデスクトップアプリ。
- 技術構成は Tauri + React + TypeScript + SQLite。
- アプリ全体で同時に開始できるタイマーは1件だけ。
- タスクとサブタスクは、開始予定日、期限、メモ、タイマー履歴、通知を持つ。
- カレンダーMVPは週表示。
- 未完了サブタスクがある親タスクも、確認後であれば完了できる。
- タスク/サブタスク削除時は、関連するタイマー履歴もソフト削除する。
- 通知はデフォルトでタイトルのみ表示し、設定で汎用メッセージへ切り替えられる。
- GitHubはソースコード、Issue、Pull Request、Release管理に使う。
- アプリ実行時に外部API、分析、リモートフォント、リモート画像、自動更新エンドポイントへ接続しない。

## ドキュメント

- [MVP仕様](docs/mvp-spec.md)
- [アーキテクチャ](docs/architecture.md)
- [ドメインモデル](docs/domain-model.md)
- [データベーススキーマ](docs/database-schema.sql)
- [セキュリティ設計](docs/security.md)
- [テスト戦略](docs/testing.md)
- [運用方針](docs/operations.md)
- [リリース前チェックリスト](docs/release-checklist.md)
- [設定方針](docs/configuration.md)
- [実装計画](docs/implementation-plan.md)
- [次の作業](docs/next-actions.md)
- [レビューチェックリスト](docs/review/checklist.md)
- [ADR 0001: デスクトップ技術構成](docs/adr/0001-desktop-stack.md)
- [ADR 0002: オフライン優先ローカル保存](docs/adr/0002-offline-first-local-storage.md)
- [ADR 0003: 単一アクティブタイマー](docs/adr/0003-single-active-timer.md)

## 開発ルール

実装は次の順序で進めます。

1. 仕様
2. 設計
3. レビュー
4. 実装

対象ユースケース、トランザクション境界、入力検証、セキュリティ影響が説明できるまで実装を始めません。

## リポジトリ状態

このリポジトリには、設計資料、運用設定、Tauri + Reactの初期プロジェクト構成が入っています。

## ローカル開発

依存関係をインストールした後に起動します。

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

アプリ実行時はオフライン前提です。依存関係のインストールやGitHub Actionsは開発時の通信であり、アプリ実行時の外部通信ではありません。

## リリース運用

GitHub Actionsの `リポジトリチェック` は、PRとブランチpushで基本チェックを実行します。

配布形式:

- macOS: `dmg`
- Windows: `nsis`

リリース前には [リリース前チェックリスト](docs/release-checklist.md) を使い、macOS/Windowsの手動確認、通知権限、オフライン起動、外部通信なしの方針を確認します。

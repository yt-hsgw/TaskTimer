# ソースAPIリファレンス

TaskTimerのソースコードリファレンスは、実装言語に対応する標準的な生成器から作成します。手書きのKDoc風資料は持たず、ソースコメントと型定義を正とします。

## 生成方式

| 対象 | コメント形式 | 生成器 | 出力 |
| --- | --- | --- | --- |
| TypeScript / React | TSDoc互換の `/** ... */` | TypeDoc | `target/docs/typescript/index.html`、`target/docs/typescript.json` |
| Rust / Tauri | rustdocの `//!`、`///` | `cargo doc` / rustdoc | `src-tauri/target/doc/tasktimer_lib/index.html` |

生成物はビルド成果物として扱い、Git管理しません。設計判断は `docs/`、機械可読な全体構造は `docs/handoff/tasktimer-structure.json`、APIの責務と制約はソースコメントに記録します。

TypeDoc 0.28.20はTypeScript 7をまだpeer dependencyとして受け付けないため、`tools/api-docs` workspaceだけTypeScript 6.0.3を使用します。アプリ本体はTypeScript 7のままです。TypeDocがTypeScript 7へ正式対応した時点で、分離workspaceを廃止できるか再評価します。

## 実行コマンド

```bash
npm run docs
```

個別に生成する場合:

```bash
npm run docs:typescript
npm run docs:rust
```

## コメント方針

- モジュール先頭には、そのレイヤー、責務、依存してはいけない対象を記載する。
- 公開する型、Use Case境界、Repository境界、外部副作用アダプターには、入力、出力、不変条件、副作用を記載する。
- 実装から明らかな説明を繰り返さず、設計上の理由と破ってはいけない制約を優先する。
- タスク名、メモ、通知本文など、ユーザーが入力した実データを例やログへ含めない。
- 予定期間 `scheduled_*` と通知期限 `due_*`、通常タイマーとポモドーロを混同しない。
- TypeScript 7固有構文を導入する場合は、TypeDoc用TypeScript 6でも解析できるかCIで確認する。解析不能ならTypeDoc正式対応を待つか生成toolchainを見直す。

## 参照順序

1. `docs/handoff/tasktimer-structure.json` で対象レイヤーと主要ファイルを特定する。
2. `docs/handoff/tasktimer-blueprint.html` で画面とデータフローを俯瞰する。
3. このページの手順でAPIリファレンスを生成し、型とモジュール責務を確認する。
4. 詳細な設計判断は `docs/architecture.md`、`docs/domain-model.md`、ADRを確認する。

## 更新ルール

API、責務、不変条件、副作用、権限境界を変更した場合は、同じ変更内で該当ソースコメントを更新します。CIはTypeDocとrustdocを生成し、コメント内リンクの破損や生成不能を検出します。

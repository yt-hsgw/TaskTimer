# 035: Rust静的解析CIの実行時間を短縮する

GitHub Issue: #94

## 目的

Rustの静的解析CI、特に `cargo clippy` を含むリポジトリチェックの待ち時間を短縮する。

## 現状

`リポジトリチェック` workflowは1つの `verify` jobで以下を直列実行している。

- Rust toolchain、rustfmt、clippyの準備。
- Linuxビルド依存関係のインストール。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- TypeScript/Vite build、実行時プライバシー監査、公開運用向け静的確認。

2026-07-15時点の直近main実行では、job全体が約4分05秒、Rust testが約2分12秒、Rust clippyが約1分17秒だった。

## スコープ

- `Swatinem/rust-cache@v2` を導入し、Cargo registry、Cargo git、`src-tauri/target` の再利用を狙う。
- Rust品質基準は維持する。
- キャッシュの効き具合をGitHub Actions上で確認できるよう、設計資料とPRへBefore/Afterを残す。

## スコープ外

- テスト削減。
- `cargo clippy --all-targets -- -D warnings` の緩和。
- Rustコードのリファクタリングによるコンパイル対象削減。
- `mold`、`lld`、`sccache` の即時導入。

## 設計レビュー

### 方針

まずビルドキャッシュを導入する。Rust依存と `target` 成果物の再利用は品質基準を変えずに効果を出しやすく、PRごとの待ち時間を下げる第一手として安全性が高い。

`fmt`、`test`、`clippy` のジョブ分割は今回は見送る。現在のworkflowは1ジョブ内で `cargo test` の後に `cargo clippy` を実行しており、同じ `target` をその場で再利用できる。分割すると実時間は短くなる可能性がある一方、Linuxビルド依存関係のインストールと初回コンパイルが複数jobで重複する可能性がある。

### トランザクション境界

アプリ実行時の挙動、データモデル、Repository、Use Caseには影響しない。変更対象はGitHub Actionsと運用資料に限定する。

### セキュリティ

Workflow permissionsは `contents: read` のまま維持する。追加するcache actionは開発・運用時の依存キャッシュであり、アプリ実行時の外部通信やTauri権限を増やさない。

### トレードオフ

- キャッシュ導入でPR再実行や継続開発時のRustビルド短縮が期待できる。
- 初回実行や `Cargo.lock` 更新直後はキャッシュ効果が小さい。
- キャッシュ復元/保存自体の時間が増えるため、効果はActions上で実測する。

### 代替案

`fmt`、`test`、`clippy` を別jobへ分割して並列化する。

不採用理由:

- 現在の規模では依存インストールとコンパイルの重複が増える可能性がある。
- まずキャッシュ導入後の実測を見て、分割が必要か判断する方が安全である。

## 受け入れ条件

- `リポジトリチェック` workflowにRustビルドキャッシュが入っている。
- `cargo test` と `cargo clippy --all-targets -- -D warnings` が維持されている。
- `permissions: contents: read` が維持されている。
- PR本文またはコメントにBefore/AfterのActions実行時間が記録されている。
- 運用資料にRust CIキャッシュの注意点が記載されている。

## 危険ケース

- キャッシュ導入のために `-D warnings` を外してしまう。
- 失敗時の古い成果物を信じ、依存更新後の問題を見落とす。
- キャッシュのためにSecretや追加権限を増やしてしまう。
- Actions上の短縮効果を測らず、複雑な高速リンカーやsccacheを追加してしまう。

## フォローアップ候補

- cache warm後もclippyが支配的であれば、`fmt`、`test`、`clippy` のジョブ分割を再検討する。
- リンク時間が支配的であれば、Linux runner限定で `lld` または `mold` を検討する。
- 複数workflow間で同じRust成果物を共有したい場合は `sccache` を検討する。

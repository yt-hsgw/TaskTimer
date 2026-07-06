# [Ops] リリース運用とGitHub管理を整える

## 目的

GitHub上でIssue/PR/Releaseを管理し、Windows/macOS向けの配布準備を進める。

## 対応内容

- GitHubリポジトリを作成する。
- `origin` remoteを設定する。
- Issue labelを整備する。
- GitHub Actionsでビルド/スキーマ確認を実行する。
- macOS/Windowsの配布形式を決める。
- 署名・インストール手順・既知制限を文書化する。

## 完了条件

- GitHub Issuesで残作業を追跡できる。
- PRテンプレートが運用されている。
- Actionsで基本チェックが動く。
- リリース前手動確認手順がある。

## 実装方針

- PRと `main` pushで `リポジトリチェック` を実行する。
- CIでは設計ファイル、SQLiteスキーマ、初期マイグレーション、Rust format/test/clippy、TypeScript/Vite build、秘密情報ファイル、空白エラーを確認する。
- OS固有の通知権限、インストーラー、署名警告はCIでは保証しない。`docs/release-checklist.md` とリリースIssueテンプレートで手動確認する。
- 配布形式は現在のTauri設定に合わせ、macOSは `dmg`、Windowsは `nsis` とする。
- 自動更新artifactはMVPでは作成しない。

## 設計理由

- アプリ本体はオフライン実行が前提だが、GitHub Actionsでの依存取得と検証は開発・運用時の通信として分離できる。
- パッケージ生成や署名をCIへ入れる前に、署名方針と配布対象を別ADRで固める必要がある。
- リリース手順をIssueテンプレート化すると、OS別の手動確認結果をGitHub上で追跡できる。

## トレードオフ

- 基本チェックをCIに増やすため、PRの待ち時間は長くなる。
- OS別パッケージ生成まで自動化しないため、リリース作業には手動手順が残る。
- 署名なしartifactではOS警告が出る可能性がある。

## 代替案

- macOS/Windowsのパッケージ生成をGitHub Actions matrixへ追加する。自動化は進むが、署名、artifact保管、OS通知の実機確認を同時に設計する必要があるためMVPでは見送る。

## 危険ケース

- ローカルだけで作業が進み、Issueと実装が乖離する。
- CIでは通るがOS別パッケージングで失敗する。
- 外部通信禁止の方針がリリース設定で崩れる。

## ラベル候補

- `documentation`
- `ops`
- `priority: P2`

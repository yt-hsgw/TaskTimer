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

## 危険ケース

- ローカルだけで作業が進み、Issueと実装が乖離する。
- CIでは通るがOS別パッケージングで失敗する。
- 外部通信禁止の方針がリリース設定で崩れる。

## ラベル候補

- `documentation`
- `ops`
- `priority: P2`


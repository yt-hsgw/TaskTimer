# 007: 外部利用者向けGitHub運用

## 目的

外部の利用者がGitHubからTaskTimerを入手し、利用条件を理解し、安全に問い合わせできる運用にする。

## スコープ

- MIT Licenseへの切り替え。
- READMEの利用者向け導線追加。
- GitHub Releases向けRelease workflow追加。
- `CONTRIBUTING.md`、`SUPPORT.md`、`CHANGELOG.md` の追加。
- IssueとDiscussionsの役割整理。
- 公開運用ADRと公開運用資料の追加。

## スコープ外

- 自動更新。
- 署名と公証。
- ストア配布。
- Linux配布。
- アプリ実行時の外部通信。

## 設計レビュー

### データモデル

変更なし。

### トランザクション境界

アプリのDBトランザクション変更なし。GitHub運用上は、タグ作成、Release workflow、Draft Release公開を境界として扱う。

### 権限境界

- 通常CIは `contents: read` を維持する。
- Release workflowのみ `contents: write` を使う。
- アプリ本体にネットワーク権限、自動更新権限、分析通信を追加しない。

### セキュリティ

- Issue、Discussions、Release notesへ実データやDBを投稿しない注意を明記する。
- Release artifactはDraftで確認してから公開する。
- 署名なしartifactの警告を既知制限として記載する。

## トレードオフ

- MIT Licenseは外部利用しやすいが、再配布や商用利用も許可する。
- GitHub Releasesは導入が軽いが、署名済みストア配布ほどの信頼性はない。
- Draft Release運用は安全だが、公開前の手動確認作業が増える。

## 代替案

- All rights reservedを維持してバイナリだけ配布する。利用許諾が狭く、外部貢献と再利用がしづらい。
- Apache-2.0を採用する。特許許諾が明確だが、MVPでは説明負荷が増える。
- ストア配布を先に整える。利用者の信頼性は上がるが、署名、審査、運用コストが増える。

## 受け入れ条件

- READMEからReleases、Issues、Discussions、Security Policyへ移動できる。
- LICENSEがMIT Licenseである。
- Release workflowがWindows/macOSのDraft Releaseを作成できる。
- 公開運用資料とADRに理由、トレードオフ、危険ケースがある。
- CIの必須ファイルチェックに新しい運用ファイルが含まれる。

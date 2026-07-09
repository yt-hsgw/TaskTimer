# 019: 次の作業リストを現状に合わせて更新する

GitHub Issue: #20

## 目的

UI/UX改修Issueが完了したあとも `docs/next-actions.md` に古いUI作業が最優先として残っているため、次に取るべき作業をリリース前ゲート中心へ更新する。

## 背景

UI/UX関連の主要Issueは完了済みである。

- #25 タスク一覧を新UIへ置き換える。
- #26 左ナビゲーションとApp Shellを実装する。
- #27 UI/UX用データモデルとRead Modelを整備する。
- #28 カレンダーと設定を左ナビ配下へ移管する。
- #29 右詳細ペインを実装する。
- #30 タイマー一時停止/再開と繰り返し設定を実装する。

現在のOpen IssueはRelease/運用系の #20、#22、#24 であり、次の作業リストもこの状態へ合わせる必要がある。

## スコープ

- `docs/next-actions.md` の最優先項目をRelease前ゲートへ更新する。
- UI/UX改修済み項目を完了済みに移す。
- post-v0.1.0改善候補をMVPリリース前作業と分離する。
- 危険ケースを現在のRelease/運用リスクへ更新する。

## スコープ外

- UI実装の追加変更。
- Release workflowの実行。
- GitHub Secretsへの実値登録。
- Open Issueのクローズ。

## 設計レビュー

### データモデル

アプリのドメインデータ、SQLiteスキーマ、Repository、Use Caseは変更しない。変更対象は作業計画ドキュメントのみである。

### トランザクション境界

- Docs更新: 次に着手すべき作業の認識を固定する境界。
- Release workflow実行: 署名・公証済みartifactを生成する境界。
- Draft Release公開: 外部利用者へ配布を開始する境界。

### セキュリティ

- GitHub SecretsやApple認証情報の実値は書かない。
- 未解決のglib advisoryは #22 で追跡し、Linux artifactを配布しない方針を維持する。
- macOS署名・公証preflightとGatekeeper実機確認をRelease前ゲートとして明示する。

## トレードオフ

- リリース前作業を最優先に寄せると、post-v0.1.0の改善候補は後ろに下がる。
- ただし、外部利用者に配布する前は、UI追加より署名・公証、artifact鮮度、実機確認の方がリスク低減に効く。

## 代替案

古いUIタスクを残したままにする。

不採用理由:

- GitHub Issue上は完了済みなのに資料上は未完了に見え、次に着手すべき作業を誤る。

## 破綻シナリオ

- UI改修が未完了だと誤認して、Release前ゲートよりUI微調整を優先してしまう。
- macOS署名・公証Secrets未登録のままDraft Releaseを公開してしまう。
- 古いDraft artifactを公開して、Release notesと実際のアプリ挙動が食い違う。
- glib advisoryの扱いをRelease notesから漏らす。

## 受け入れ条件

- `docs/next-actions.md` の最優先がRelease前ゲート中心になっている。
- UI/UX主要Issueが完了済みとして記録されている。
- post-v0.1.0改善候補がMVP公開前作業と分離されている。
- 秘密情報や個人情報を含まない。

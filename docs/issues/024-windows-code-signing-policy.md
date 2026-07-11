# 024: Windowsコード署名方針を決める

GitHub Issue: #50

## 目的

Windows先行Releaseを継続しつつ、コード署名未設定によるSmartScreen警告と将来の署名導入境界を明確にする。

## スコープ

- Windowsコード署名の採用方針または保留判断をADRへ記録する。
- Release notesに残す既知制限の表現を定義する。
- 将来導入する場合のGitHub Secrets、workflow、確認手順の境界を定義する。
- コスト、本人確認、証明書更新、SmartScreen評価のリスクを記録する。

## スコープ外

- Windowsコード署名の実装。
- 証明書購入、Azure設定、Secret登録。
- Microsoft Store配布。
- macOS署名・公証。これはGitHub #24で追跡する。

## 設計レビュー

### データモデル

アプリのデータモデル、SQLiteスキーマ、Repository境界には影響しない。

### トランザクション境界

Application Use Caseのトランザクション境界には影響しない。
署名はRelease artifact作成時の運用境界であり、DBコミットや通知副作用とは独立する。

### セキュリティ

証明書、秘密鍵、証明書パスワード、Azure認証情報はリポジトリ、Issue、PR、Release notes、Actionsログへ書かない。
署名導入時はGitHub SecretsまたはGitHub Environment Secretsだけを秘密情報の保存先にする。

### 破綻シナリオ

- 署名済みならSmartScreen警告が必ず消えると誤説明する。
- 自己署名証明書を公開配布に使う。
- Secret値をログやGitHub本文へ出す。
- 証明書更新時の評価リセットや失効を運用に含めない。

### スケール

v0.1.xは未署名配布を継続し、署名導入を別Issueへ分離することで、Release作業の複雑さを増やさない。
将来導入時は、署名検証、Windows実機確認、証明書更新期限の定期確認をRelease運用へ追加する。

## トレードオフ

- 未署名配布は利用者に警告が出る可能性があるが、費用とSecret運用を先送りできる。
- Azure Artifact SigningはCI/CDに統合しやすい一方、対象地域と本人確認の制約がある。
- OV証明書は従来型で選択肢が広い一方、HSMまたはハードウェアトークン運用が必要になる可能性がある。

## 代替案

署名を導入するまでWindows Releaseを止める。

不採用理由:

- v0.1.0はWindows実機確認済みであり、既知制限を明記すれば利用者が判断できる。
- 署名導入には費用、本人確認、Secret管理、更新作業が必要で、別Issueとして扱う方が安全。

## 受け入れ条件

- [ADR 0005](../adr/0005-windows-code-signing-policy.md) に方針が記録されている。
- README、運用資料、リリース前チェックリスト、Release notesの既知制限がADRと矛盾しない。
- 将来導入する場合のSecret境界とworkflow境界が文書化されている。

## レビュー判断

フォローアップ付き承認。

- v0.1.xでは未署名Windows artifactを既知制限付きで配布する。
- Windowsコード署名導入は別Issueで、Secrets、workflow、確認手順を設計してから実装する。

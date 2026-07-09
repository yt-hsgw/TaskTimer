# 018: macOS署名・公証preflightを追加する

GitHub Issue: #24

## 目的

macOS署名・公証済みDMGを生成する前に、必要なGitHub Actions Secrets、Tauri設定、Entitlements、ローカル検証ツールの状態を確認できるようにする。

## 背景

Issue #24では、Developer ID署名とApple公証によってGatekeeperの「Appleが検証できません」警告を軽減する必要がある。

Release workflowはmacOSジョブ内でSecrets不足を検出するが、Secrets登録前にworkflowを走らせると、Release作業の途中で失敗する。公開前の手動ゲートとしてpreflightを用意し、実値をログへ出さずに不足しているSecrets名だけを確認する。

## スコープ

- macOS署名・公証用GitHub Actions Secretsの名前が登録済みか確認する。
- TauriのmacOS bundle設定が署名・公証前提になっているか確認する。
- Entitlementsに不要なネットワーク、ファイル、カメラ、マイク、位置情報権限が含まれないことを確認する。
- macOS上では `codesign`、`security`、`notarytool`、`stapler` の存在を確認する。
- Release checklistとIssue #24設計メモへpreflight手順を追加する。

## スコープ外

- Apple Developer Programの契約。
- Developer ID Application証明書の発行。
- GitHub Secretsへの実値登録。
- Secrets値、証明書、Apple ID、App用パスワードの表示または保存。
- Release workflowの実行。
- Gatekeeper実機確認。

## 実装方針

- `scripts/check-macos-signing-preflight.mjs` を追加し、`npm run check:macos-signing` から実行できるようにする。
- GitHub Secretsは `gh secret list --app actions --json name,updatedAt` で名前だけ確認する。
- Secrets値は取得しない。ログにも出さない。
- macOS以外ではOS固有ツール確認をwarningとしてスキップし、Secretsとリポジトリ設定の確認は実行する。
- preflightが失敗した場合は、Draft Releaseを公開しない判断材料として扱う。

## 設計レビュー

### データモデル

アプリのドメインデータ、SQLiteスキーマ、Repository、Use Caseは変更しない。変更対象はRelease運用のpreflight境界のみである。

### トランザクション境界

- Preflight: Release workflow実行前に、設定とSecrets名の準備状態を確認する境界。
- Release workflow: 署名・公証済みartifactを生成する境界。
- Gatekeeper実機確認: 生成artifactの配布可否を判断する境界。

Preflightが失敗した場合は、Release workflowを実行しても署名・公証済みartifact生成に進めない可能性が高いため、Draft Release公開を止める。

### セキュリティ

- Secrets値は読み取らない。
- Secrets名だけを確認し、ログに出すのは不足している名前だけにする。
- Entitlementsに不要な権限を追加しない。
- Preflightは署名・公証の準備確認であり、入力検証やローカルデータ保護の代替ではない。

### 権限境界

- GitHub CLIはRepository Secretsのメタデータを読むために使う。
- Release workflowは引き続き `contents: write` のみを要求する。
- Preflight scriptはtag、Release、Secrets値を変更しない読み取り専用の手順として扱う。

### スケール

Secrets数は固定で少ない。macOS Intel/Apple Siliconの両方で同じRepository Secretsを参照するため、preflightは1回で足りる。

## トレードオフ

- workflow実行前にpreflightを追加すると手順は増えるが、Secrets不足によるRelease失敗を早く検出できる。
- GitHub CLIに依存するためローカル環境準備が必要だが、既存のIssue/PR運用でも `gh` を利用している。
- Secrets値の妥当性までは確認できないが、値を出力しないことで漏えいリスクを抑える。

## 代替案

Release workflowのSecrets検証だけに任せる。

不採用理由:

- 署名・公証の準備不足をRelease workflow実行時まで検出できず、Draft Release作成作業の手戻りが大きい。

Secrets値をローカルで検証する。

不採用理由:

- 証明書やApple認証情報をローカルログやIssueへ露出するリスクが上がる。MVPでは名前の存在確認とRelease workflowの実行結果で十分にする。

## 破綻シナリオ

- Secrets未登録のままRelease workflowを実行し、macOSジョブが失敗する。
- Entitlementsへ不要なネットワーク権限を追加し、外部通信なしの方針と矛盾する。
- Secrets値をIssue、PR、Release notes、ログへ貼ってしまう。
- Preflight成功だけで署名・公証済みと誤認し、Gatekeeper実機確認を省略する。

## 受け入れ条件

- `npm run check:macos-signing` を実行できる。
- GitHub Actions Secrets不足時は不足名だけを表示して失敗する。
- Tauri macOS bundle設定とEntitlementsの危険な権限を確認できる。
- Release checklistにpreflight手順がある。
- Issue #24は、Secrets登録、Release workflow成功、Gatekeeper実機確認が終わるまで継続追跡として残る。

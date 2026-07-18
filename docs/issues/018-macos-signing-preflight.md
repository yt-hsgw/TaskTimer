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
- Secretsに依存しない設定検査モードを通常CIで実行する。
- Release build後に `.app` の署名、Gatekeeper評価、公証チケットと、`.dmg` の署名を検証する。
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
- `--configuration-only` ではGitHub SecretsとローカルmacOSツールの確認を省略し、通常CIから設定回帰を検出する。
- GitHub Secretsは `gh secret list --app actions --json name,updatedAt` で名前だけ確認する。
- Secrets値は取得しない。ログにも出さない。
- macOS以外ではOS固有ツール確認をwarningとしてスキップし、Secretsとリポジトリ設定の確認は実行する。
- preflightが失敗した場合は、Draft Releaseを公開しない判断材料として扱う。
- Release workflowはTauri Actionの `artifactPaths` JSONを成果物検証スクリプトへ渡す。スクリプトはリポジトリ配下に存在する `.app` と `.dmg` だけを検証対象にする。
- `.app` は `codesign --verify --deep --strict`、`spctl --assess --type execute`、`xcrun stapler validate` で検証する。
- `.dmg` は `codesign --verify` でDeveloper ID署名と改変がないことを検証する。Tauri標準フローではDMG内の `.app` が公証・staple対象である。

## 設計レビュー

### データモデル

アプリのドメインデータ、SQLiteスキーマ、Repository、Use Caseは変更しない。変更対象はRelease運用のpreflight境界のみである。

### トランザクション境界

- Preflight: Release workflow実行前に、設定とSecrets名の準備状態を確認する境界。
- Release workflow: 署名・公証済みartifactを生成する境界。
- 成果物検証: 生成artifactを公開候補として受け入れる境界。
- Gatekeeper実機確認: 生成artifactの配布可否を判断する境界。

Preflightが失敗した場合は、Release workflowを実行しても署名・公証済みartifact生成に進めない可能性が高いため、Draft Release公開を止める。

### セキュリティ

- Secrets値は読み取らない。
- Secrets名だけを確認し、ログに出すのは不足している名前だけにする。
- Entitlementsに不要な権限を追加しない。
- Tauri Actionの出力はJSONとして解析し、コマンド文字列として評価しない。
- 検証対象をリポジトリ配下の `.app` と `.dmg` に限定する。
- Preflightは署名・公証の準備確認であり、入力検証やローカルデータ保護の代替ではない。

### 権限境界

- GitHub CLIはRepository Secretsのメタデータを読むために使う。
- Release workflowは引き続き `contents: write` のみを要求する。
- Preflight scriptはtag、Release、Secrets値を変更しない読み取り専用の手順として扱う。
- 成果物検証scriptは生成済みartifactを読み取るだけで、署名、公証、Release内容を変更しない。

### スケール

Secrets数は固定で少ない。macOS Intel/Apple Siliconの両方で同じRepository Secretsを参照するため、preflightは1回で足りる。成果物検証はアーキテクチャごとに2対象だけを確認し、アプリ内データ量には依存しない。

## トレードオフ

- workflow実行前にpreflightを追加すると手順は増えるが、Secrets不足によるRelease失敗を早く検出できる。
- GitHub CLIに依存するためローカル環境準備が必要だが、既存のIssue/PR運用でも `gh` を利用している。
- Secrets値の妥当性までは確認できないが、値を出力しないことで漏えいリスクを抑える。
- 成果物検証はTauri ActionがDraftへartifactを添付した後になるが、失敗時にDraft公開を禁止することで配布を止める。

## 代替案

Release workflowのSecrets検証だけに任せる。

不採用理由:

- 署名・公証の準備不足をRelease workflow実行時まで検出できず、Draft Release作成作業の手戻りが大きい。

Secrets値をローカルで検証する。

不採用理由:

- 証明書やApple認証情報をローカルログやIssueへ露出するリスクが上がる。MVPでは名前の存在確認とRelease workflowの実行結果で十分にする。

成果物を検証してから独自処理でReleaseへuploadする。

不採用理由:

- Tauri ActionのRelease生成、命名、upload処理を再実装する必要があり、権限境界と保守対象が増える。
- Draft Releaseを公開しない運用で外部配布を止められるため、現段階では標準Actionの出力を検証する構成を採用する。

## 破綻シナリオ

- Secrets未登録のままRelease workflowを実行し、macOSジョブが失敗する。
- Entitlementsへ不要なネットワーク権限を追加し、外部通信なしの方針と矛盾する。
- Secrets値をIssue、PR、Release notes、ログへ貼ってしまう。
- Preflight成功だけで署名・公証済みと誤認し、Gatekeeper実機確認を省略する。
- 署名後に改変された `.app` または `.dmg` を公開する。
- 公証チケットがstapleされていない `.app` を含むDMGを、オフライン環境へ配布する。

## 受け入れ条件

- `npm run check:macos-signing` を実行できる。
- GitHub Actions Secrets不足時は不足名だけを表示して失敗する。
- Tauri macOS bundle設定とEntitlementsの危険な権限を確認できる。
- 通常CIでSecretsなしの設定検査が成功する。
- Release workflowで `.app` の署名・公証チケットと `.dmg` の署名検証が成功するまでDraftを公開しない。
- Release checklistにpreflight手順がある。
- Issue #24は、Secrets登録、Release workflow成功、Gatekeeper実機確認が終わるまで継続追跡として残る。

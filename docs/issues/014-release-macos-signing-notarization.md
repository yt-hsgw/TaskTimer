# 014: macOS署名と公証を設定する

GitHub Issue: #24

## 目的

GitHub Releasesから配布するmacOS DMGをDeveloper ID署名とApple公証済みにし、外部利用者がGatekeeper警告で起動を阻まれにくい状態にする。

## スコープ

- TauriのmacOSバンドル設定を明示する。
- GitHub ActionsのRelease workflowでmacOS署名・公証用Secretsを必須化する。
- Release notesとリリース前チェックリストから、macOS未署名artifactの既知制限を更新する。
- 秘密情報の保存場所と権限境界を文書化する。

## スコープ外

- Apple Developer Programの契約、証明書発行、App Store Connect API Key発行。
- GitHub Secretsへの実値登録。
- 実機でのGatekeeper警告解消確認。
- Windowsコード署名。
- Mac App Store配布。

## 実装方針

- `src-tauri/tauri.conf.json` の `bundle.macOS.hardenedRuntime` を有効にする。
- `src-tauri/Entitlements.plist` は空のdictとし、外部通信やユーザーデータアクセスの権限を追加しない。
- 署名IDはリポジトリに固定値を書かず、`APPLE_SIGNING_IDENTITY` Secretから渡す。
- Release workflowのmacOSジョブでは、署名・公証Secretsが未設定の場合にFail-fastする。
- 公証認証はMVPではApple ID方式を正とし、`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID` を使う。
- Release workflow実行前に `npm run check:macos-signing` でTauri設定、Entitlements、GitHub Actions Secrets名、macOS検証ツールをpreflightする。
- 通常CIでは `npm run check:macos-signing-config` を実行し、Secretsが未登録でもTauri設定とEntitlementsの回帰を検出する。
- macOS Release build後はTauri Actionの `artifactPaths` 出力を検証スクリプトへ渡し、生成した `.app` の署名、Gatekeeper評価、公証チケットと、`.dmg` の署名を検証する。
- 成果物検証に失敗したDraft Releaseは公開しない。Draftに添付済みの成果物も配布対象として扱わない。

## 必要なGitHub Secrets

- `APPLE_CERTIFICATE`: Developer ID Application証明書を `.p12` でexportし、base64化した値。
- `APPLE_CERTIFICATE_PASSWORD`: `.p12` export時のパスワード。
- `APPLE_SIGNING_IDENTITY`: `security find-identity -v -p codesigning` で確認した署名ID。
- `APPLE_ID`: 公証に使うApple ID。
- `APPLE_PASSWORD`: Apple IDのApp用パスワード。
- `APPLE_TEAM_ID`: Apple Developer Team ID。

## 設計レビュー

### データモデル

アプリのドメインデータ、SQLiteスキーマ、ユーザー設定は変更しない。変更対象は配布artifact生成の運用モデルのみ。

### トランザクション境界

- Git tag作成: リリース対象コミットを固定する境界。
- Release workflow: 署名・公証済みartifactを生成する境界。
- macOS成果物検証: 生成済みartifactを公開候補として受け入れる境界。
- Draft Release公開: 外部利用者へ配布を開始する境界。

macOS署名、公証、成果物検証のいずれかに失敗した場合はDraft Releaseを公開しない。

### セキュリティ

- 証明書、証明書パスワード、Apple認証情報はGitHub Secretsにのみ保存する。
- Secrets値をリポジトリ、Issue、PR、ログに出さない。
- Entitlementsに不要なネットワーク、ファイル、Keychain権限を追加しない。
- 成果物パスはTauri ActionのJSON出力だけを入力にし、シェル文字列として評価しない。
- 成果物検証では署名者名やTeam IDの実値を受け入れ条件へ固定せず、Developer ID Application署名であることだけを確認する。
- 署名・公証は配布元の信頼性を高める仕組みであり、入力検証やローカルデータ保護の代替にしない。

### 破綻シナリオ

- Secrets未設定のままmacOS Releaseを実行し、未署名DMGを公開してしまう。
- 個人の署名IDや証明書をリポジトリに書き込み、秘密情報または個人情報として漏えいする。
- Entitlementsへネットワーク権限を追加し、外部通信なしの利用者期待を壊す。
- 公証失敗を見落としてDraft Releaseを公開する。
- ad-hoc署名または署名後に改変されたartifactを、Developer ID署名済みと誤認する。
- 公証チケットがstapleされていないアプリをDMGへ含め、オフライン環境でGatekeeperが公証状態を確認できない。
- Windows未署名警告をmacOSの署名対応で解消済みと誤認する。

### スケール

macOS IntelとApple Siliconの2ジョブで同じSecretsを使う。成果物検証は各ジョブの `.app` と `.dmg` に対して一定回数だけ実行するため、タスク件数やDB容量には依存しない。Secretsはリポジトリ単位で管理し、証明書更新時はworkflow変更ではなくSecrets更新で対応する。

## トレードオフ

- Apple ID方式は導入が単純だが、App用パスワードとTeam IDの管理が必要。
- App Store Connect API Key方式はCI向けに分離しやすいが、`.p8` キーファイルの安全な配置設計が追加で必要。
- Secrets未設定でmacOSジョブを失敗させると一時的にRelease作成が止まるが、未署名artifactを配布する事故を防げる。
- Tauri ActionはbuildとDraft Releaseへの添付を同じstepで行うため、成果物検証は添付後になる。失敗時にDraftを公開しない運用を必須にする代わりに、Release生成処理の独自実装は避ける。

## 代替案

App Store Connect API Key方式を採用する。

不採用理由:

- MVPではApple ID方式の方が必要Secretsが少なく、GitHub Actions内で秘密鍵ファイルを生成する処理も不要。
- API Key方式へ切り替える場合は、`APPLE_API_KEY`、`APPLE_API_ISSUER`、`APPLE_API_KEY_PATH` の扱いを別Issueで設計する。

Ad-hoc署名を使う。

不採用理由:

- Gatekeeperの「Appleが検証できない」警告を根本的に解消できず、外部利用者向け配布の目的に合わない。

Tauri buildとGitHub Releaseへのuploadを別jobへ分離する。

不採用理由:

- 検証前artifactをDraftへ添付しない構成にできる一方、2アーキテクチャのartifact受け渡しと単一Draft Releaseへの集約処理が増える。
- Draftは外部公開境界ではないため、現段階では検証失敗時に公開を禁止する方が構成と権限を小さく保てる。

## 受け入れ条件

- TauriのmacOS署名前提設定が明示されている。
- Release workflowでmacOS署名・公証Secretsが必須化されている。
- Release workflowがTauri buildへ署名・公証環境変数を渡している。
- Secretsに依存しないmacOS署名設定チェックが通常CIで成功する。
- macOS Release jobが `.app` のDeveloper ID署名、Gatekeeper評価、staple済み状態を検証する。
- macOS Release jobが、公証済み `.app` を含む `.dmg` のDeveloper ID署名を検証する。
- `docs/release-checklist.md` に署名・公証確認項目がある。
- 秘密情報をコード、Issue、PR、ログへ出さない運用が文書化されている。

## 残る手動作業

- Apple Developer ProgramでDeveloper ID Application証明書を発行する。
- 証明書を `.p12` でexportしてbase64化し、GitHub Secretsへ登録する。
- App用パスワードとTeam IDをGitHub Secretsへ登録する。
- `npm run check:macos-signing` でpreflightが成功することを確認する。
- `app-v*` タグまたは手動実行でRelease workflowを走らせ、生成DMGを実機で開いてGatekeeper警告が解消されることを確認する。
- GitHub ActionsのmacOS成果物検証stepがApple Silicon、Intelの両方で成功することを確認する。

## 参考

- Tauri v2 macOS Code Signing: https://v2.tauri.app/distribute/sign/macos/
- Tauri v2 Environment Variables: https://v2.tauri.app/reference/environment-variables/

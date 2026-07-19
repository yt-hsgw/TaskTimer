# macOS署名・公証成果物検証レビュー

## 対象

- GitHub Issue #24
- `src-tauri/tauri.conf.json`
- `src-tauri/Entitlements.plist`
- `.github/workflows/release.yml`
- macOS署名・公証preflightと成果物検証

## 指摘事項

### 重要: build後の成果物を検証していない

Release workflowは署名・公証用環境変数をTauriへ渡しているが、生成した `.app` のDeveloper ID署名と公証チケット、および `.dmg` のDeveloper ID署名を確認していない。Secretsの誤値、期限切れ証明書、Tauri設定回帰があっても、build stepの結果だけで公開判断する余地がある。

対応:

- Tauri Actionの `artifactPaths` JSONを入力にして、`.app` と `.dmg` の存在とリポジトリ配下であることを検証する。
- `.app` の署名、Developer ID Application署名者、Gatekeeper評価、公証チケットを検証する。
- 公証・staple済み `.app` を含む `.dmg` のDeveloper ID署名を検証する。
- いずれかが失敗したDraft Releaseを公開しない。

### 中: 設定検査がGitHub Secretsの準備状態に結合している

既存の `npm run check:macos-signing` はGitHub Secrets未登録時に失敗するため、通常CIへ追加できない。Tauri設定やEntitlementsの回帰をPR時に検出できない。

対応:

- SecretsとOSツールを確認しない `--configuration-only` モードを追加する。
- 通常CIでは設定検査を必須化し、Release前には従来どおり完全preflightを実行する。

## 設計判断

### 状態と副作用

- 設定検査はリポジトリファイルの読み取りだけを行う。
- 完全preflightはGitHub Secretsの名前とローカルツールの存在だけを読み取る。
- 成果物検証は生成済みファイルを読み取り、署名、公証、Release、Secretsを変更しない。
- Draft Releaseの公開は人が行う別の副作用境界とする。

### トランザクション境界

1. Release tagで対象commitを固定する。
2. Tauri buildが署名・公証済みartifactを生成する。
3. 成果物検証がartifactを公開候補として受け入れる。
4. 実機確認後にDraft Releaseを公開する。

2または3が失敗した場合は4へ進まない。

## セキュリティレビュー

- 証明書、パスワード、Apple ID、Team IDの値をスクリプトへ引数として渡さない。
- Secretsの確認は登録名だけに限定する。
- artifact pathはJSONとして解析し、シェル評価しない。
- 検証対象はリポジトリ配下に存在する `.app` と `.dmg` に限定する。
- 空のEntitlementsを維持し、外部通信、ファイル、カメラ、マイク、位置情報の権限を追加しない。
- workflow権限は既存の `contents: write` を超えて追加しない。

## 破綻シナリオ

- Secrets名は存在するが証明書またはApp用パスワードが無効で、公証に失敗する。
- ad-hoc署名をDeveloper ID署名と誤認する。
- 署名後にartifactが改変される。
- `.app` の公証またはstapleに失敗したまま、配布用 `.dmg` が生成される。
- Tauri Actionの出力形式が変わり、検証対象を特定できない。
- Intelだけ、またはApple Siliconだけ成果物検証に失敗する。

すべてRelease workflowを失敗させ、Draft公開を止める。

## スケール

検証対象は各macOS buildにつき `.app` と `.dmg` の2件で固定される。タスク件数、サブタスク件数、タイマー履歴、SQLite容量には依存しない。IntelとApple Siliconは独立jobで検証する。

## テスト方針

- Linuxの通常CIで設定検査モードを実行する。
- 不正な引数、壊れたJSON、対象不足、リポジトリ外pathを自動テストする。
- macOSのRelease workflowでApple標準ツールによる実artifact検証を行う。
- 最終的なGatekeeper表示は、別MacへDMGをダウンロードして手動確認する。

## 代替案

Tauri build、成果物検証、Release uploadを別jobへ分離する案がある。検証前artifactをDraftへ添付しない利点はあるが、2アーキテクチャのartifact受け渡しと単一Releaseへの集約処理が増える。Draftは未公開なので、現時点では標準Tauri Actionを維持し、検証失敗時の公開禁止を採用する。

## 判断

フォローアップ付き承認。

フォローアップ:

- GitHub Actions Secretsへ実値を登録する。
- macOSを含むRelease workflowを実行し、両アーキテクチャの成果物検証を成功させる。
- 別MacでGatekeeper警告が解消されることを確認する。

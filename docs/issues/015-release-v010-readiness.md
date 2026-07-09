# 015: v0.1.0公開判定資料を整える

GitHub Issue: #20

## 目的

v0.1.0をGitHub Releasesから外部利用者へ配布する前に、Release notes、既知制限、手動確認、ロールバック判断をリポジトリ上でレビューできる状態にする。

## スコープ

- v0.1.0用のRelease notes草案を作成する。
- CHANGELOGとREADME/Release運用資料の表現を矛盾しない状態にする。
- Release Issueへ転記すべき手動確認項目と公開ブロック条件を明確にする。
- 既知制限として、Windowsコード署名未設定、Linux未配布、glib advisory追跡、macOS配布後回しを明記する。

## スコープ外

- Git tagの作成または付け替え。
- Draft Releaseの公開。
- GitHub Secretsへの実値登録。
- Apple Developer Programでの証明書発行。
- Windowsコード署名の導入。
- Linux artifactの配布。

## 実装方針

- `docs/releases/v0.1.0.md` をRelease notesの正本として扱い、GitHub Release本文へ転記できる構成にする。
- `CHANGELOG.md` は利用者に見える変更履歴として、公開判定待ちの状態と既知制限を短く表現する。
- Release checklistとIssue templateは、Release notes草案、既知制限、Dependabot追跡Issueの確認を必須項目にする。
- GitHub Actionsの自動Release本文は、草案との差し替え前でも最低限の利用者向け情報が残るようにする。

## 設計レビュー

### データモデル

アプリのドメインデータ、SQLiteスキーマ、Repository、Use Caseは変更しない。今回の変更は配布運用とドキュメントの境界に限定する。

### トランザクション境界

- Release notes草案のPRマージ: 外部利用者向け説明を `main` に固定する境界。
- `app-vX.Y.Z` tag作成: ビルド対象コミットを固定する境界。
- Release workflow完了: 配布artifactを生成しDraft Releaseへ添付する境界。
- Draft Release公開: 外部利用者がダウンロードできる状態へ移す境界。

Draft Release公開前にWindows手動確認が失敗した場合は、公開せずRelease Issueへ結果を記録する。macOS artifactを配布する場合は、署名・公証確認も公開前ゲートに含める。

### セキュリティ

- Release notes、Issue、PRにはApple証明書、Apple ID、App用パスワード、Team ID、ローカルDB、ログ、個人タスク内容を書かない。
- アプリ実行時の外部通信なし、自動更新なし、ローカル保存方針をRelease notesに明記する。
- 既知の依存関係アラートは隠さず、配布対象と影響範囲を説明する。

### 権限境界

- GitHub Repository Secretsは署名・公証の秘密情報を保持する境界。
- Release workflowは全体で `contents: read` を基本とし、artifactを添付するjobだけ `contents: write` を要求する。
- GitHub Release公開は、Draftの手動確認後に行う運用権限として扱う。

## トレードオフ

- Release notesをリポジトリに置くとPRレビューしやすい一方、GitHub Release本文へ転記する運用が増える。
- Draft Release本文だけで管理すると転記は不要だが、変更理由、既知制限、レビュー履歴がコードレビューに残りにくい。
- Windows先行配布にすると公開は早まるが、macOS利用者向けの正式artifact提供は遅れる。

## 代替案

GitHub Release本文だけを正本にする。

不採用理由:

- Draft Release本文の差分は通常のPRレビューに乗らず、README、CHANGELOG、Release checklistとの矛盾を見落としやすい。

自動生成Release notesだけにする。

不採用理由:

- 既知制限、手動確認結果、外部通信なし、自動更新なし、署名・公証状態など、外部利用者が判断に必要な説明を十分に固定できない。

## 破綻シナリオ

- Windows先行Releaseなのに、macOS artifact提供済みであるようにRelease notesへ書いてしまう。
- macOS artifactを配布する場合に、署名・公証未確認のDMGを公開して、Gatekeeper警告で利用者が起動できない。
- Windowsコード署名未設定をRelease notesに書かず、SmartScreen警告を不具合として受け取られる。
- Dependabot alert #1を隠したまま公開し、Linux配布を追加した時に影響範囲を誤る。
- Release Issueに個人のタスク内容、SQLite DB、スクリーンショット、Apple認証情報を書いてしまう。
- 旧コミットのDraft Release notesを更新せず、現在の機能や既知制限とずれたまま公開する。

## 受け入れ条件

- v0.1.0用Release notes草案がある。
- CHANGELOGのv0.1.0とUnreleasedが現在の配布方針と矛盾しない。
- Release checklistとIssue templateでRelease notes草案、既知制限、Dependabot追跡Issueを確認できる。
- 秘密情報をリポジトリ、Issue、PR、Release notesに書かない注意が残っている。
- 実装変更なしであることを確認できる。

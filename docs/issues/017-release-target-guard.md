# 017: v0.1.0のRelease target検証を追加する

GitHub Issue: #20

## 目的

Draft Releaseを公開する前に、Release tagとDraft artifactが意図したリリース対象コミットから作られていることを確認できるようにする。

## 背景

2026-07-09時点で、既存のDraft Release `app-v0.1.0` は `bf13f1c` を対象に作られている。一方で、`main` はその後のPRマージにより `cfd0753` まで進んでいる。

この状態で既存Draft Releaseを公開すると、README、Release notes、依存関係監視、UI/UX改善など、後からマージされた変更が入っていないartifactを外部利用者へ配布してしまう。

## スコープ

- Release tagが意図したコミットを指しているかを確認する手動スクリプトを追加する。
- Release checklistとIssue templateに、tagとDraft artifactの鮮度確認を追加する。
- v0.1.0 Release notes草案へ、既存Draft Releaseをそのまま公開しない注意を追加する。

## スコープ外

- Draft Releaseの削除。
- `app-v0.1.0` tagの付け替え。
- Release workflowの自動実行。
- macOS署名・公証Secretsの登録。
- macOS/Windowsの実機インストール確認。

## 実装方針

- `scripts/check-release-target.mjs` は `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` のversion一致を確認する。
- Release tagは `app-vX.Y.Z` 形式とし、指定したtarget refのcommitと一致するかを確認する。
- 既定では現在のプロジェクトversionと `HEAD` を比較し、手動運用では `origin/main` を明示して使う。
- スクリプトは読み取り専用とし、tagやReleaseを変更しない。

## 設計レビュー

### データモデル

アプリのドメインデータ、SQLiteスキーマ、Repository、Use Caseは変更しない。変更対象はRelease運用の検証境界のみである。

### トランザクション境界

- Version整合確認: Release対象versionを固定する前提条件。
- Tag作成: Release対象コミットを固定する境界。
- Draft Release生成: tagに紐づくartifactを作る境界。
- Draft Release公開: 外部利用者がダウンロードできる状態へ移す境界。

Tag対象と公開対象がずれている場合は、Draft Releaseを公開しない。

### セキュリティ

- 古いartifactを公開すると、既知制限や監視強化が反映されない可能性がある。
- スクリプトはGit履歴だけを読み取り、秘密情報、DB、個人データには触れない。
- GitHub Secrets、Apple認証情報、証明書値は扱わない。

### スケール

Release公開前に1回実行するだけなのでCI負荷はない。複数versionを扱う場合も、version引数とtarget refを変えるだけで再利用できる。

## トレードオフ

- 手動ゲートとして追加するため、自動化より運用者の確認作業は残る。
- CIに常時組み込むと、既存tagが意図的に古い場合やPRブランチ上で失敗しやすいため、Release公開前の手動確認に限定する。
- Draft Releaseを毎回作り直す運用は手間が増えるが、古いartifactを公開する事故を避けやすい。

## 代替案

Release workflow内で常に `main` とtagの一致を強制する。

不採用理由:

- 緊急修正版や過去commitからのReleaseを作る余地を失う。MVPでは手動チェックで対象commitを明示する方が柔軟である。

GitHub Release本文だけに注意を書く。

不採用理由:

- 実際のtagが古いかどうかを機械的に検証できず、公開直前の見落としを防ぎにくい。

## 破綻シナリオ

- 旧commitのDraft Releaseを公開し、READMEやRelease notesとartifactの挙動が食い違う。
- tag付け替え後にDraft Release artifactを作り直さず、古いartifactだけが残る。
- version番号は一致しているが、`main` の最新修正がartifactに含まれていない。
- macOS署名・公証後のartifactとRelease notesの手動確認結果が別commitを指す。

## 受け入れ条件

- 手元で `npm run check:release-target -- 0.1.0 <target-ref>` を実行できる。
- Release checklistにtagとDraft artifactの鮮度確認がある。
- v0.1.0 Release notes草案に既存Draft Releaseをそのまま公開しない注意がある。
- スクリプトがtagやReleaseを変更しない。

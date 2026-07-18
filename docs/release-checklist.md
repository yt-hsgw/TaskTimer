# リリース前チェックリスト

## 目的

Windows向けにTaskTimerを配布する前に、実務運用で必要な品質、セキュリティ、オフライン方針を確認する。macOS配布はApple署名・公証準備が完了したReleaseでだけ対象に含める。

このチェックリストは、GitHub Releaseを作成する前の手動ゲートとして使う。GitHub Actionsは基本的なビルドとテストを確認するが、OS通知やインストーラー挙動はOSごとの手動確認を必須とする。

## リリース単位

- バージョンは `src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`package.json` の整合を確認する。
- 配布形式はTauri設定に合わせる。
- Windows: `nsis`。
- macOS: `dmg`。Apple署名・公証準備が完了したReleaseでのみ対象に含める。
- GitHub Release tagは `app-vX.Y.Z` 形式にする。
- Release tagは意図したリリース対象commitを指している必要がある。
- GitHub Actionsの `リリースビルド` でDraft Releaseを作成する。
- `リリースビルド` は既定でWindows artifactだけを作成する。
- macOS artifactを作成する場合は、手動実行で `include_macos` を有効にする。
- `npm run check:release-platform-policy` が成功し、Linuxまたは未知のartifactターゲットがない。
- 自動更新用artifactはMVPでは作成しない。`createUpdaterArtifacts` は `false` を維持する。
- macOS artifactを配布する場合はDeveloper ID署名とApple公証を必須にする。
- Windowsコード署名はv0.1.xでは未導入のため、Release notesに既知制限として記載する。判断は [ADR 0005](adr/0005-windows-code-signing-policy.md) に従う。

## 事前条件

- `main` がリリース対象のコミットを指している。
- 関連IssueとPRがGitHub上で追跡できる。
- 未解決の `priority: P0` または `priority: P1` Issueがない。
- `docs/public-readiness.md` を確認済みである。
- `docs/public-operations.md` を確認済みである。
- `docs/releases/<version>.md` のRelease notes草案を確認済みである。
- リリースノートに既知制限、手動確認結果、外部通信方針を記載できる。
- 未解決またはdismiss済みのDependabot alertにリスク受容がある場合は、影響範囲、配布対象、ADRをRelease notesに記載できる。
- Windows artifactを実機でインストール確認できる環境がある。
- macOS artifactを配布する場合は、macOS署名・公証用GitHub Secretsを登録済みである。
- Secrets値をIssue、PR、Release notes、ログに貼っていない。

## Windowsコード署名方針

v0.1.xではWindowsコード署名を導入せず、未署名Windows artifactを既知制限付きで配布する。

Release notesには次を記載する。

- Windows版はコード署名未設定である。
- Windows SmartScreenまたは組織ポリシーにより警告やブロックが表示される場合がある。
- GitHub Releasesの公式URL、Release notes、SHA-256を確認してから利用判断する。

Windowsコード署名を導入する場合は、別IssueでSecrets、workflow、確認手順、証明書更新手順を設計してから実装する。証明書、秘密鍵、証明書パスワード、Azure認証情報はリポジトリ、Issue、PR、Release notes、Actionsログへ出さない。

Release tagとtarget commitを確認する。

```bash
git fetch origin main --tags
npm run check:release-target -- <version> origin/main
```

既存Draft Releaseが古いcommitのartifactを持っている場合は、Draft Releaseを公開せず、Release notesと手動確認結果を引き継いだうえでDraft Releaseとtagを作り直す。

## macOS署名・公証Secrets

このセクションはmacOS artifactを配布対象に含める場合だけ必須とする。WindowsのみのReleaseではApple準備を後回しにできる。

GitHub Repository Secretsに以下を登録する。

- `APPLE_CERTIFICATE`: Developer ID Application証明書を `.p12` でexportし、base64化した値。
- `APPLE_CERTIFICATE_PASSWORD`: `.p12` export時のパスワード。
- `APPLE_SIGNING_IDENTITY`: `security find-identity -v -p codesigning` で確認した署名ID。
- `APPLE_ID`: 公証に使うApple ID。
- `APPLE_PASSWORD`: Apple IDのApp用パスワード。
- `APPLE_TEAM_ID`: Apple Developer Team ID。

Secrets値はローカルファイル、Issue、PR、Release notesに保存しない。

macOS込みRelease workflow実行前にpreflightを実行する。

```bash
npm run check:macos-signing
```

このコマンドはSecrets値を読み取らず、登録済みSecret名とTauri設定、Entitlements、macOS検証ツールの存在だけを確認する。失敗した場合はmacOS artifactを配布しない。

通常CIではSecretsを必要としない設定検査を実行する。

```bash
npm run check:macos-signing-config
```

このコマンドはTauri設定、空のEntitlements、Release workflowの署名・公証経路だけを確認する。Secretsの登録状態と実値の妥当性、成果物の署名状態は保証しない。

## 自動チェック

GitHub Actionsで以下が成功していることを確認する。

- Release対象決定job。
- 必須設計ファイルの存在確認。
- SQLiteスキーマと初期マイグレーションの検証。
- Rust format check。
- Rust unit/integration test。
- Rust clippy。
- npm audit。
- TypeScript/Vite build。
- 公開済みReleaseまたはpre-releaseに対するWindows runnerでのインストーラー最低限検証。
- macOS artifactを含める場合のmacOS署名・公証Secrets検証。
- Secretsに依存しないmacOS署名・公証設定検査。
- macOS artifactを含める場合の `.app` と `.dmg` の署名・公証成果物検証。
- `.env` と `.env.*` の誤コミット検出。
- DB、鍵、証明書、ログ、個人環境パス、メールアドレスの誤コミット検出。
- `git diff --check`。

ローカルで確認する場合:

```bash
npm ci
sqlite3 :memory: ".read docs/database-schema.sql"
sqlite3 :memory: ".read src-tauri/migrations/0001_initial.sql"
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run build
npm run audit:runtime-privacy
npm run check:macos-signing-config
npm run test:release-scripts
git diff --check
```

## 手動デスクトップ確認

Windowsでは必ず確認する。macOSはartifactを配布対象に含める場合だけ確認する。

- インターネット接続なしでアプリが起動する。
- タスクを作成、完了、削除できる。
- サブタスクを作成、完了、削除できる。
- タスクタイマーを開始、停止できる。
- サブタスクタイマーを開始、停止できる。
- 同時に開始できるタイマーが1件だけである。
- タイマー開始中にアプリを再起動しても、アクティブタイマーが復元される。
- GitHub Actionsの `Windows復帰回帰検証` が成功している。
- Windowsをスリープ/復帰してTaskTimerを前面に戻したとき、アクティブタイマーと通知状態が再同期される。
- カレンダーの週/日/月表示で期限のあるタスク/サブタスクが表示される。
- カレンダーで予定期間付きタスクを作成し、週/日/月表示で期間ブロックを確認できる。
- 予定期間の開始端/終了端をドラッグと矢印キーで調整でき、期限通知は変更されない。
- カレンダーのサブタスク項目に親タスク名が表示される。
- 通知全体OFF中に期限到来タスク/サブタスクを作成してもOS通知が送信されない。
- 通知全体ONへ戻すと、既存の期限到来通知を設定画面から再試行できる。
- アプリを開いたまま2分後の期限時刻を持つタスクを作成し、予定時刻にOS通知が届く。
- 未来時刻通知が届いたあと、再フォーカスや画面更新で同じ通知が重複送信されない。
- 通知全体OFFへ変更したあと、既存の未来時刻通知が予定時刻になっても送信されない。
- 通知表示タイプ `タイトルのみ` でメモ本文が通知に出ない。
- 通知表示タイプ `汎用メッセージ` でタスク/サブタスクタイトルが通知に出ない。
- 通知権限拒否時に設定画面から失敗と再試行導線が分かる。
- OSスリープ/復帰後にタイマーと通知状態が破綻しない。
- 期限到来通知が復帰または再フォーカス後に重複送信されない。

macOS artifactを配布する場合だけ確認する。

- macOS DMGを開いたときに「Appleが検証できません」警告が出ない。
- macOSで `spctl --assess --type execute --verbose /Applications/TaskTimer.app` が成功する。
- macOSで `xcrun stapler validate /Applications/TaskTimer.app` が成功する。

## セキュリティ確認

- アプリ実行時の外部通信を追加していない。
- リモートフォント、リモート画像、分析、クラッシュアップロード、自動更新通信を追加していない。
- `npm run audit:runtime-privacy` が成功している。
- `npm run check:release-platform-policy` が成功している。
- Linux限定`glib` advisoryのリスク受容と再審査条件がADR 0007に記録されている。
- 新しいTauri権限を追加した場合は、PR本文と設計資料に理由がある。
- タスク名、サブタスク名、メモ本文、通知本文をログに出していない。
- メモ本文をHTMLとして描画していない。
- 秘密情報、DBファイル、個人データをRelease artifactやIssue/PRに添付していない。
- SQLiteバックアップ、JSONエクスポート、CSVエクスポートをRelease artifactやIssue/PRに添付していない。
- Apple証明書、Apple ID、App用パスワード、Team IDをログやGitHub本文へ出していない。
- Windows署名用の証明書、秘密鍵、証明書パスワード、Azure認証情報をログやGitHub本文へ出していない。
- Git履歴の著者情報を公開してよいか確認済みである。

## リリース作成手順

1. `main` のGitHub Actionsが成功していることを確認する。
2. Windowsの手動確認結果をリリースIssueへ記録する。macOS artifactを配布する場合はmacOSの手動確認結果も記録する。
3. `app-vX.Y.Z` タグを `main` の対象コミットへ作成してpushする。またはGitHub Actionsから `リリースビルド` を手動実行する。
4. Draft ReleaseへWindows artifactが添付されることを確認する。
5. `npm run check:release-target -- <version> origin/main` でtagと公開対象commitが一致することを確認する。
6. Windows実機確認を完了できない場合は通常Releaseとして公開せず、Release notesに未確認範囲と配布判断を明記する。
7. `Windowsインストーラー検証` workflowを対象tagで手動実行し、Windows runner上のサイレントインストール/アンインストールが成功することを確認する。
8. macOS artifactを配布する場合は、手動実行で `include_macos` を有効にし、`npm run check:macos-signing` と `preflight-macos` が成功することを確認する。
9. macOS artifactを配布する場合は、macOSジョブで署名・公証が成功していることを確認する。
10. macOS artifactを配布する場合は、Apple SiliconとIntelの両jobで `macOS署名・公証成果物を検証` が成功していることを確認する。
11. Windowsでは生成された `nsis` artifactを手動でインストール確認する。macOS artifactを配布する場合は生成された `dmg` も手動確認する。
12. `docs/releases/<version>.md` の草案をもとに、Release notesへ変更点、既知制限、手動確認結果、外部通信なしの方針を記載する。
13. Windowsコード署名未設定によるSmartScreenまたは組織ポリシーの警告可能性を既知制限に記載する。
14. 未解決またはdismiss済みのDependabot alertにリスク受容がある場合は、影響範囲、配布対象、ADRを既知制限に記載する。
15. 通常Releaseとして扱える確認が完了したら、pre-releaseを解除して公開する。

ローカルでartifactを作る場合:

```bash
npm run tauri:build
```

## ロールバック

- 重大な不具合が見つかった場合、該当Releaseをpre-releaseまたはdraftへ戻す。
- GitHub Issueに影響範囲、再現手順、回避策を記録する。
- DBマイグレーションを含む不具合では、データ復旧方針を確認するまで修正版を公開しない。

## 破綻シナリオ

- CIは通るが、WindowsまたはmacOS固有の通知権限で通知が届かない。
- インストール済みアプリでは通知表示名やアイコンが開発時と異なる。
- WindowsだけのReleaseなのに、Release notesがmacOS artifact提供済みであるように見える。
- Windows runnerのインストール検証成功を、通知やGUIを含む実機確認完了と誤認する。
- macOS artifactを含める時に、macOS署名・公証Secretsが未設定でRelease workflowが失敗する。
- macOS署名・公証preflightの失敗を無視してmacOS artifactを公開する。
- macOS成果物検証stepの失敗を無視してDraft Releaseを公開する。
- 公証が失敗したDMGを公開してしまい、Gatekeeper警告により業務利用者へ配布できない。
- Windows未署名artifactがOSセキュリティ警告により業務利用者へ配布しにくい。
- 署名済みartifactならSmartScreen警告が必ず消えると誤説明する。
- OS復帰または再フォーカス時に同じ期限通知が重複送信される。
- リリースノートに既知制限がなく、利用者が通知やタイマー復元の仕様を誤解する。
- 依存関係アラートの影響範囲をRelease notesに書かず、利用者が配布対象OSとリスクを判断できない。
- 古いcommitで生成したDraft Release artifactを公開し、Release notesや手動確認結果と実artifactが食い違う。
- `.env.*` や個人DBファイルをartifactまたはIssueへ添付してしまう。

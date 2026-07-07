# パブリック公開前チェック

## 目的

GitHubリポジトリを公開する前に、秘密情報、個人情報、意図しないライセンス許諾、依存関係管理、外部通信方針を確認する。

この文書はリポジトリ公開前の運用チェックであり、アプリ実行時の仕様変更ではない。

## 現時点の判断

- 追跡ファイルに `.env`、ローカルDB、秘密鍵、証明書、ログファイルを含めない。
- 追跡ファイルに個人環境の絶対パスやメールアドレスを含めない。
- GitHub Issue/PR本文に秘密情報や個人データを貼らない。
- 依存関係更新はDependabotで追跡する。
- OSSライセンスはMIT Licenseを採用する。判断は [ADR 0004](adr/0004-public-distribution-license.md) に記録する。
- アプリ実行時の外部通信禁止方針は維持する。GitHub Actions、Dependabot、依存関係取得は開発・運用時の通信として扱う。

## 公開前に実行する確認

```bash
git ls-files | grep -E '(^|/)(\.env(\..*)?|.*\.(db|sqlite|sqlite3|pem|key|p12|mobileprovision|log))$'
git grep -n -E '(/[U]sers/|[A-Za-z]:\\[U]sers\\|/[h]ome/[^/]+/)' -- . ':!package-lock.json' ':!src-tauri/Cargo.lock'
git grep -n -E '[[:alnum:]._%+-]+@[[:alnum:].-]+\.[[:alpha:]]{2,}' -- . ':!package-lock.json' ':!src-tauri/Cargo.lock'
npm audit --audit-level=moderate
```

期待結果:

- 最初の3コマンドは何も出力しない。
- `npm audit` はmoderate以上の既知脆弱性を報告しない。

## CIで確認すること

`リポジトリチェック` では、通常のビルド/テストに加えて以下を確認する。

- 公開前に必要な運用ファイルが存在する。
- npm依存関係にmoderate以上の既知脆弱性がない。
- `.env`、ローカルDB、秘密鍵、証明書、ログファイルが追跡されていない。
- 追跡ファイルに個人環境の絶対パスが含まれていない。
- 追跡ファイルにメールアドレスが含まれていない。

## GitHub上で確認すること

- リポジトリのVisibility変更前に、Open/ClosedのIssueとPR本文へ秘密情報がないことを確認する。
- GitHubのPrivate Vulnerability Reporting、またはSecurity Advisoryの利用可否を確認する。
- GitHub Secret scanningとDependabot alertsを有効にする。
- GitHub Discussionsを質問窓口として使える状態にする。
- GitHub Releasesに署名なしartifactの既知制限を記載する。
- 公開後にGitHub Actionsが成功することを確認する。

## Git履歴の注意

GitHubでリポジトリを公開すると、コミット履歴の著者名と著者メールも見える。

通常のメールアドレスを含む過去コミットがある場合、公開前に次のどちらかを選ぶ。

- そのまま公開する。
- 履歴を書き換えてnoreplyメールへ置換し、force pushする。

履歴書き換えは既存ブランチ、PR、clone済み作業ツリーへ影響するため、明示承認なしでは実施しない。

## 設計理由

- パブリック公開はアプリのランタイム機能ではなく、リポジトリ運用上の権限境界である。
- 依存関係の自動監視は公開後のリスク低減に効くが、アプリ実行時の外部通信ではない。
- MIT Licenseを採用することで、外部利用者と貢献者の利用条件を明確にする。

## トレードオフ

- MIT Licenseにより外部利用はしやすくなるが、商用利用や再配布も許可される。
- Dependabot PRが増えるため、定期的なレビュー負荷が生じる。
- Git履歴を書き換えない場合、過去コミットの著者情報は公開される。

## 代替案

- Apache-2.0を採用する。特許許諾が明確になるが、MIT Licenseよりライセンス文と説明が重くなる。
- All rights reservedを維持する。再配布を制限しやすいが、外部利用者が扱いにくい。
- Dependabotを使わず手動で依存更新する。PRノイズは減るが、脆弱性対応の検知が遅れる。

## 危険ケース

- `.env`、ローカルDB、秘密鍵、証明書、ログを誤って追跡する。
- IssueやPRに個人のタスク内容やスクリーンショットを貼る。
- README、LICENSE、ADRのライセンス方針が矛盾する。
- Git履歴の著者メールを公開後に隠したくなり、後から履歴書き換えが必要になる。
- Dependabot更新を未確認でマージし、Tauri権限や外部通信方針が変わる。

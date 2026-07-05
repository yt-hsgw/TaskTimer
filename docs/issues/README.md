# Issue草案

このディレクトリは、GitHub remote設定前のIssue草案置き場です。

現状、このローカルリポジトリにはGitHub remoteが未設定のため、`gh issue create` で実Issueを作成できません。GitHubリポジトリ作成後に、各Markdownの内容をGitHub Issueへ移してください。

## 草案一覧

- [001 SQLite接続とRepository境界を実装する](001-sqlite-connection-repository-boundary.md)
- [002 コアUse Caseを実装する](002-core-usecases.md)
- [003 UIをDB接続へ切り替える](003-connect-ui-to-db.md)
- [004 タイマー完了・削除・復元の境界ケースを固める](004-timer-delete-edge-cases.md)
- [005 ローカル通知を実装する](005-local-notification.md)
- [006 リリース運用とGitHub管理を整える](006-release-and-github-operations.md)

## GitHub Issue化の前提

1. GitHub上にリポジトリを作成する。
2. このローカルリポジトリに `origin` を設定する。
3. 必要なlabelを作成する。
4. 各草案を `gh issue create` またはGitHub UIで登録する。


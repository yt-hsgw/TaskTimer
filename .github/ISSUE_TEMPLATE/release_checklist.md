---
name: リリースチェック
about: リリース前の自動確認と手動確認を記録する
title: "[Release] v"
labels: ops,documentation
assignees: ""
---

## リリース対象

- バージョン:
- 対象コミット:
- 対象OS:

## 自動チェック

- [ ] GitHub Actions: リポジトリチェック
- [ ] GitHub Actions: リリースビルド
- [ ] SQLiteスキーマ検証
- [ ] SQLiteマイグレーション検証
- [ ] Rust format
- [ ] Rust test
- [ ] Rust clippy
- [ ] TypeScript/Vite build
- [ ] 秘密情報ファイル誤コミット確認

## Windows手動確認

- [ ] オフライン起動
- [ ] タスク作成/完了/削除
- [ ] サブタスク作成/完了/削除
- [ ] タスクタイマー開始/停止
- [ ] サブタスクタイマー開始/停止
- [ ] 同時に開始できるタイマーが1件だけ
- [ ] 週カレンダー表示
- [ ] 通知表示タイプ `タイトルのみ`
- [ ] 通知表示タイプ `汎用メッセージ`
- [ ] 通知権限拒否時の失敗表示と再試行
- [ ] インストーラーartifact確認

## セキュリティ確認

- [ ] アプリ実行時の外部通信を追加していない
- [ ] ユーザー内容をログに出していない
- [ ] ユーザー内容をHTMLとして描画していない
- [ ] 新しいTauri権限の理由を記録した
- [ ] 秘密情報、DBファイル、個人データを添付していない
- [ ] Windows署名用の証明書、秘密鍵、証明書パスワード、Azure認証情報を本文やログへ出していない

## 配布判断

- [ ] GitHub Release tagが `app-vX.Y.Z` 形式
- [ ] Release tagが意図したリリース対象commitを指している
- [ ] Draft Releaseとして作成されている
- [ ] Draft Release artifactがRelease tagと同じcommitから生成されている
- [ ] Windows artifactを実機インストール確認した
- [ ] `docs/releases/<version>.md` のRelease notes草案を確認した
- [ ] Release notesに変更点を記載した
- [ ] Release notesに既知制限を記載した
- [ ] Release notesに外部通信なしと自動更新なしを記載した
- [ ] 未解決のDependabot alertがある場合、影響範囲、配布対象、追跡Issueを既知制限に記載した
- [ ] Windowsコード署名未設定によるSmartScreenまたは組織ポリシーの警告可能性を既知制限に記載した
- [ ] Release notesにmacOSは現時点で配布対象外であることを記載した
- [ ] ロールバック判断基準を確認した

## メモ

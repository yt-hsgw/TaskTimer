## 概要

-

## 対象範囲

-

## 設計レビュー

- 仕様を更新または確認した:
- データモデルへの影響:
- トランザクション境界への影響:
- 検討した代替案:
- トレードオフ:
- 破綻シナリオ:

## セキュリティレビュー

- アプリ実行時の外部通信を追加したか: いいえ / はい:
- ユーザー内容をログに出すか: いいえ / はい:
- ユーザー内容をHTMLとして描画するか: いいえ / はい:
- 新しい権限を追加したか: いいえ / はい:
- 入力検証を更新したか: いいえ / はい:
- 秘密情報または個人データを含むファイルを追加したか: いいえ / はい:

## 公開運用レビュー

- README、LICENSE、Release notes、運用資料に矛盾がないか:
- GitHub ReleasesやActionsの権限に変更があるか:
- 外部利用者へ知らせる既知制限があるか:

## テスト証跡

- [ ] SQLiteスキーマ検証
- [ ] SQLiteマイグレーション検証
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] `npm run build`
- [ ] `git diff --check`
- [ ] デスクトップ手動確認:

## 確認した危険ケース

-

# セキュリティ設計

## セキュリティ目標

TaskTimerは、すべての実行時データをローカルに保持し、アプリ権限を最小化することでユーザーのタスク内容を保護する。

## 権限境界

許可すること:

- アプリデータディレクトリ内のローカルSQLiteデータベース。
- ローカルOS通知。
- 明示的なユーザー操作に基づく、アプリ所有データ、ログ、エクスポート/インポートファイルへのローカルアクセス。

MVPで禁止すること:

- アプリ実行時の外部API呼び出し。
- 分析送信。
- クラッシュアップロード。
- リモート画像。
- リモートフォント。
- 自動更新のネットワーク呼び出し。
- リモートオリジンの埋め込みWebコンテンツ。

## OWASP観点レビュー

| リスク | 設計上の対策 |
| --- | --- |
| Injection | SQLiteクエリは必ずパラメータ化する。 |
| XSS相当の描画事故 | メモとタイトルはHTMLではなくテキストとして描画する。 |
| Broken access control | Tauri commandは明示ユースケースのみを公開し、生のファイル/DBアクセスを公開しない。 |
| Security misconfiguration | 未使用のTauri権限とネットワーク機能を無効化する。 |
| Vulnerable components | 依存関係追加前にレビューし、依存面を小さく保つ。 |
| Sensitive data exposure | タスク名、メモ、通知本文、ユーザー識別子を含むDBパスをログに出さない。 |
| Integrity failure | データベースマイグレーションはバージョン管理し、テストする。 |
| SSRF/network misuse | アプリ実行時のネットワークアクセスを設計に含めない。 |

## バックアップ/エクスポートプライバシー

- SQLiteバックアップ、JSONエクスポート、CSVエクスポートは個人データとして扱う。
- バックアップ/エクスポートには、タスク名、サブタスク名、メモ本文、タイマー履歴、通知ルールが含まれる可能性がある。
- 公開Issue、PR、Discussions、Release artifactへバックアップ/エクスポートファイルを添付しない。
- 復元時のエラー表示とログには、タスク名、メモ本文、通知本文、ファイル内容を含めない。
- 詳細は [ローカルデータのバックアップとエクスポート方針](data-backup-export.md) に従う。

## 入力検証

Application境界で検証する。

- `title`: trim、空不可、長さ上限。
- `memo`: 長さ上限、プレーンテキストのみ。
- `planned_start_date`: ISO日付またはnull。
- `due_date`: ISO日付またはnull。
- `notify_at`: 妥当なISO日時。
- `target_type`: enum。
- `target_id`: UUID形式。
- `status`: enum。
- `notification_display_mode`: `title_only` または `generic`。
- `notifications_enabled`: boolean。

## 通知プライバシー

- デフォルトの通知表示モードは `title_only`。
- `title_only` はタスクまたはサブタスクのタイトルのみを表示する。
- `generic` はタスクまたはサブタスクのタイトルをOS通知adapterへ渡さず、プライバシー保護メッセージだけを表示する。
- メモ本文は通知に含めない。
- 通知表示モードと通知全体ON/OFFはローカル設定として保存する。
- 通知全体OFF時は、通知ルールを保持したままOS通知adapterへタスク名、サブタスク名、通知本文を渡さない。
- OS通知はTauri公式notification pluginをRust側adapterから呼び出す。JS側へ通知plugin権限を追加しない。

## ログルール

ログ出力してよいもの:

- 操作名。
- エラーカテゴリ。
- マイグレーションバージョン。
- 通知登録状態。

ログ出力禁止:

- 完全なタスクタイトル。
- メモ本文。
- 通知本文。
- SQL値そのもの。

## 危険ケース

- ユーザーがメモにHTMLやscript風テキストを入力し、UIが誤ってマークアップとして描画する。
- 通知登録に失敗したのに、UIでは通知が有効に見える。
- 通知設定が `generic` なのに、OS通知アダプターへタイトルが渡される。
- 将来追加した依存関係がデフォルトでネットワークアクセスを行う。
- アプリが想定外の場所へデータベースをエクスポートする。
- OSスリープ中もタイマーが進み、復帰後の経過時間がユーザーの期待とずれる。

## セキュリティレビューチェック

- 新しい権限を追加していないか。
- アプリ実行時の外部通信を追加していないか。
- ユーザー内容をログに出していないか。
- ユーザー内容をHTMLとして描画していないか。
- アプリデータディレクトリ外のファイルを読み書きしていないか。
- タイマーまたは通知のトランザクション境界に影響していないか。

## 実行時プライバシー監査

PRとRelease前ゲートでは、静的監査として次を実行する。

```bash
npm run audit:runtime-privacy
```

この監査で確認すること:

- 実行時コードに `fetch`、`XMLHttpRequest`、`WebSocket`、`EventSource`、`sendBeacon` がない。
- 実行時コードに `console.*`、`println!`、`eprintln!`、`dbg!`、`tracing::`、`log::` がない。
- 実行時コードにリモートURL、CSS import、リモートフォント/画像の入口がない。
- Tauri CSPがリモートオリジンやワイルドカードを許可していない。
- Tauri CapabilityにHTTP、Updater、Shell、Opener、WebSocket系の権限がない。
- macOS entitlementにnetwork client/serverがない。
- 直接依存に実行時通信やクラッシュ送信を導入しやすい依存がない。

対象外:

- GitHub Actions、Dependabot、npm/cargo installなどの開発・運用時通信。
- READMEスクリーンショット生成用のローカルVite/Chrome DevTools接続。
- Windows実機またはVMでのパケット監視。これはRelease前の手動確認として別途実施する。

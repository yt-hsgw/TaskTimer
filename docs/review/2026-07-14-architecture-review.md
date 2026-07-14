# アーキテクチャレビュー 2026-07-14

対象: #79 カスタムリスト管理を含む現行実装。

## 判断

フォローアップ付き承認。

Clean Architectureの依存方向は概ね保たれている。DomainはReact、Tauri、SQLite、OS APIへ依存しておらず、ApplicationはRepository traitとClock/Notification portを通じてInfrastructureへ依存している。

## 確認結果

- `TaskList` の作成、名称変更、削除はApplication Use Caseを経由しており、Presentationがトランザクション境界を決めていない。
- カスタムリスト削除時のタスク移動はSQLite Repository内の同一トランザクションで実装されている。
- リスト名の検証はDomain関数で行われ、SQL片として扱われていない。
- OS通知や外部通信に関する新しい権限は追加されていない。
- タスク一覧のRead Modelは、リスト指定時だけ絞り込み、今日/お気に入りでは全リストを取得できる。

## 実施したリファクタ

- 既定タスクリストのIDと名称をDomain定数として定義した。
- RustのApplication/InfrastructureはDomain定数を参照するようにした。
- React側もDomain型定義の定数を参照し、Presentationの `default` 直書きを削除した。

## 指摘事項

- `src-tauri/src/infrastructure/sqlite.rs` が5,000行を超えており、Repository実装、マイグレーション、Read Model、テストが1ファイルに集中している。
- `src/presentation/App.tsx` と `TaskDetailPane.tsx` も肥大化しており、画面状態、mutation、詳細編集フォームの責務が密になっている。

## 破綻シナリオ

- 既定タスクリストIDがレイヤーごとに不一致になると、リスト削除時にタスクの避難先が壊れる。
- リスト削除とタスク作成が連続した場合、有効リスト存在確認がないと削除済みリストにタスクが紐づく。
- 今日/お気に入りビューが選択リストのRead Modelだけを参照すると、カスタムリスト内のタスクが見えなくなる。

## スケール懸念

- `sqlite.rs` の肥大化により、今後タグ、かんばん、カレンダー編集を追加すると変更衝突が増えやすい。
- UI側は大量タスク時に `App.tsx` で `tasks` と `taskRows` を両方保持しているため、将来的にはビューごとのRead Model取得へ寄せる余地がある。

## セキュリティ懸念

- 現時点で新しい外部通信、秘密情報、OS権限は追加されていない。
- リスト名はReactのテキスト描画で扱われており、HTMLとして描画していない。
- 今後タグや色指定を追加する場合、任意HTML、任意CSS、任意SQL片として扱わない設計が必要。

## フォローアップ候補

- `sqlite.rs` をRepository実装、migration、mapping、testsへ段階的に分割する。
- `App.tsx` のsnapshot取得/mutation処理をPresentation用hookへ分離する。
- `TaskDetailPane.tsx` のサブタスク、タイマー、通知、基本情報フォームをコンポーネント分割する。

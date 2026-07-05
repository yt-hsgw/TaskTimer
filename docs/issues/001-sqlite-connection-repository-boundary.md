# [Infra] SQLite接続とRepository境界を実装する

## 目的

アプリ起動時にローカルSQLiteを初期化し、Application層からInfrastructure層へ依存を逆転できるRepository境界を用意する。

## 対応内容

- アプリデータディレクトリ配下にSQLiteファイルを作成する。
- 初期マイグレーションを実行する。
- `foreign_keys` と `busy_timeout` を設定する。
- Application層にRepository Interfaceを定義する。
- SQLite実装をInfrastructure層に閉じ込める。
- 週カレンダー、アクティブタイマー、通知表示設定の読み取り境界を作る。

## 完了条件

- `cargo check --manifest-path src-tauri/Cargo.toml` が通る。
- `npm run build` が通る。
- SQLiteスキーマが読み込める。
- Tauri commandがRepository経由で読み取りを行う。

## 危険ケース

- DB接続をPresentation層から直接呼ぶ。
- SQLiteファイルを予期しない場所に作る。
- マイグレーション失敗時に中途半端な状態で起動する。
- アクティブタイマー制約をApplicationだけに置き、DB制約を使わない。

## ラベル候補

- `enhancement`
- `infra`
- `priority: P1`


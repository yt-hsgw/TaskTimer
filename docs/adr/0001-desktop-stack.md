# ADR 0001: デスクトップ技術構成

## 状態

提案中

## 背景

アプリはWindows/macOSで動作し、データをローカルに保存し、OS通知に対応し、アプリ実行時の外部通信を避ける必要がある。

## 決定

Tauri + React + TypeScript + SQLite を採用する。

## 理由

- TauriはElectronより実行時フットプリントが小さい。
- ReactとTypeScriptは、対話的なタスクUIやカレンダーUIに向いている。
- SQLiteはローカル永続化とトランザクションに強い。
- Rust側のTauri commandで、ファイル、DB、通知の権限を狭く保てる。

## トレードオフ

- TauriによりRustパッケージングとcommand層の複雑さが増える。
- OS通知の挙動はWindows/macOSで差分がある。
- デスクトップCIや署名はWebアプリより複雑。

## 代替案

Electron + React + SQLite。

利点:

- Web開発者にとって立ち上げが速い。
- デスクトップ統合のエコシステムが大きい。

欠点:

- アプリサイズが大きい。
- メモリ使用量が増えやすい。
- セキュリティ上の実行時表面が広い。

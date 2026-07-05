# [UseCase] コアUse Caseを実装する

## 目的

MVPの中心操作であるタスク作成、サブタスク作成、タイマー開始、タイマー停止をApplication層に実装する。

## 対応内容

- `CreateTask` を実装する。
- `CreateSubtask` を実装する。
- `StartTimer` を実装する。
- `StopActiveTimer` を実装する。
- 入力検証をApplication境界で行う。
- トランザクション境界をUse Caseに持たせる。

## 完了条件

- 空タイトルを拒否する。
- 存在しない親タスクへのサブタスク作成を拒否する。
- 同時に2つ目のタイマーを開始できない。
- 停止時に `elapsed_seconds` が確定する。
- ドメイン/Use Caseテストが通る。

## 危険ケース

- UI側にドメインルールが漏れる。
- タイマー開始と対象状態更新が別トランザクションになる。
- 停止済みタイマーを再停止できる。

## ラベル候補

- `enhancement`
- `application`
- `priority: P1`


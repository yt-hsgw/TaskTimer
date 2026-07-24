# 中央ペインD&D・タイマーちらつき修正レビュー

対象: GitHub #187

## 結論

承認。D&Dとタイマー操作はいずれもPresentationの局所状態とApplicationの保存境界を分ける方針で進める。タイマーのDB正は維持しつつ、操作直後の表示はUse Case戻り値で反映し、不要なTaskPage再取得を避ける。

## チェックリスト結果

- [x] ユーザーに見える挙動と受け入れ条件が明確。
- [x] ドメインモデルとDBスキーマを変更しない。
- [x] Application Use Caseのトランザクション境界を変更しない。
- [x] Presentationの楽観表示と永続化副作用を分離する。
- [x] 単一アクティブタイマー制約はInfrastructure/Application側で維持する。
- [x] 外部通信、追加権限、秘密情報を追加しない。

## 破綻シナリオ

- D&Dのdropが保存APIを待つ間、元の予定片が表示され続ける。
- 月表示の週またぎ予定で `connects-before/after` が外れ、複数日予定が分断される。
- タイマー操作を詳細mutationとして扱い、一覧全体がdisabledになってちらつく。
- タイマー停止後にTaskRowの実行中表示だけ残る。
- タイマー操作連打で同時開始のように見える。

## スケール

- D&D後は表示中カレンダー範囲と必要なTaskPageだけを同期し、全Snapshotを読み直さない。
- タイマー操作ではTaskPageを再取得せず、`activeTimer` と該当行だけを更新する。
- カウントダウンの1秒更新はTaskPanelやAppの再描画に依存しない既存方針を維持する。

## セキュリティ

- 保存値は既存Use Caseの検証を通るため、PresentationはSQLやファイルパスを組み立てない。
- タスク名やメモをログへ出さない。
- 外部通信、リモートアセット、OS権限、Tauri capabilityは追加しない。

## テスト計画

- `npm run build`
- `npm run perf:ui -- --profile smoke --fail-on-warning`
- `npm run perf:ui -- --profile standard --fail-on-warning`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- `git diff --check`

## フォローアップ

- タスク行三点メニューとタイムライン蓋閉じは #188 で扱う。
- 右ペインのインライン編集は #189 で扱う。

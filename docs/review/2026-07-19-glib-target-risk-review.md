# glib advisory対象OS境界レビュー

## 対象

- GitHub Issue #22
- [ADR 0007](../adr/0007-glib-linux-target-risk-acceptance.md)
- Release artifact対象のCIガード
- `glib` advisory週次監視

## 指摘事項

- `Cargo.lock`に脆弱対象の`glib` 0.18.5が残るため、lockfile全体の監査結果だけでは配布物への到達性を判断できない。
- 現行のWindows/macOS配布物には`glib`が含まれないが、将来Linux artifactを追加するとadvisoryが到達可能になる。
- Dependabot alertをdismissした後も、上流制約の解消を検知する経路が必要である。

## 設計判断

- `cargo tree --target`でWindows/macOSへの非到達とLinuxでの残存を継続検査する。
- Release matrixを構造として検査し、Windowsと署名・公証対象macOS以外を拒否する。
- Dependabot alertは配布対象で未使用としてdismissし、Linux版では未解消であることと再審査条件をADRへ残す。
- 週次監視はIssue #22完了後も継続し、`glib` 0.20.0以上へ更新可能になった時点で失敗させる。

## 破綻シナリオ

- Linux artifactがレビューなしで追加される。
- Tauri更新によりWindows/macOSの依存グラフへ`glib`が混入する。
- 上流制約が解消してもdismiss済みalertが再確認されない。
- 一時的な依存更新検査が`Cargo.lock`を書き換えたまま終了する。

対応として、Release matrixとターゲット別依存グラフをCIで検査し、監視スクリプトは`finally`で`Cargo.lock`を復元する。

## スケール懸念

- ターゲット別`cargo tree`を4回実行するが、週次監視が中心であり通常のアプリ操作やデータ量には影響しない。
- Releaseポリシー検査は単一workflowの小さなJSON matrixだけを解析するため、CI時間への影響は無視できる。

## セキュリティ懸念

- Linuxでのadvisoryは解消していない。Linux版を配布対象へ加える場合は、ADR再審査と依存更新を必須とする。
- workflow権限は`contents: read`に限定し、アプリのTauri権限や実行時通信は変更しない。
- ユーザーデータ、秘密情報、通知内容は検査対象・ログ出力に含まれない。

## テスト

- `npm run check:release-platform-policy`: 許可済みのWindows/macOS Release matrixが通り、インメモリのLinux matrixが拒否される。
- `npm run check:glib-advisory`: Windows/macOSの依存グラフに`glib`がなく、Linuxだけに0.18.5が残り、0.20.0へ更新不能であることを確認した。
- `git diff --exit-code -- src-tauri/Cargo.lock`: 一時更新検査後にlockfileの差分がない。
- `npm run build`: TypeScript検査とVite production buildが成功した。
- `npm audit --audit-level=moderate`: npm脆弱性0件。
- `npm run audit:runtime-privacy`: 37ファイルを検査し、外部通信API、実行時ログ、リモートアセット、更新権限なし。
- `cargo test --manifest-path src-tauri/Cargo.toml`: 94件成功。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: 成功。

## チェックリスト結果

- プロダクト: ユーザー向け挙動変更なし。サポート対象と受け入れ条件をIssue資料とADRへ記録した。
- UI/UX: 変更なし。
- アーキテクチャ: ドメイン、Use Case、Repository、DB境界への変更なし。
- データ: スキーマ、マイグレーション、削除挙動への変更なし。
- セキュリティ: 配布対象、権限境界、残存リスク、再審査条件を明示した。
- 公開運用: Security、Release notes、運用資料、Release checklistを更新した。
- スケール: 週次CIの依存解析コストだけが増え、アプリ実行性能への影響はない。

## 判断

承認。Linux artifactを非対応のまま維持し、自動ガードと週次監視を継続することを条件とする。

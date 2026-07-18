# ADR 0007: glib advisoryの対象OS境界とリスク受容

## 状態

採用。

## 文脈

Dependabot alert #1（GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429）は、`glib` 0.15.0以上0.20.0未満の`VariantStrIter`実装に関するメモリ安全性問題を報告している。TaskTimerの`Cargo.lock`にはTauri 2.11.5から`gtk` 0.18.2を経由して`glib` 0.18.5が含まれる。

2026-07-19時点の最新Tauri 2.11.5でも`gtk`が`glib ^0.18`を要求するため、`glib` 0.20.0への互換な更新はできない。一方、Cargoのターゲット別依存グラフでは、`glib`はLinuxのGTK/WebKit経路だけに存在し、TaskTimerが配布するWindowsと将来の署名済みmacOS artifactには含まれない。

## 決定

- Dependabot alert #1は、現在の配布対象で脆弱依存が使用されないため`not_used`としてdismissする。
- Release workflowの許可対象をWindowsとmacOSに限定し、Linuxまたは未知のartifactターゲットをCIで拒否する。
- Linux版はサポート・配布しない。Linux配布を検討する場合は、このADRを再審査し、`glib` 0.20.0以上へ更新できるまでartifactを公開しない。
- 週次の`glib` advisory再評価は継続する。上流制約が解消した場合はworkflowを失敗させ、`Cargo.lock`更新とこのリスク受容の解除を促す。
- Windows、macOS、Linuxのターゲット別依存グラフを監視スクリプトで検査し、Windows/macOSへ`glib`が混入した場合は失敗させる。
- 強制的なCargo patchや互換性を無視した依存上書きは行わない。

## 設計理由

- lockfileに存在することと、配布artifactへリンクされることを分けて評価できる。
- 更新不能なalertを無期限に開く代わりに、適用範囲、配布境界、再評価条件を自動検査できる。
- alertをdismissしても週次監視を残すことで、上流修正を取り込む機会を失わない。

## トレードオフ

- GitHubの通常画面ではdismiss済みになるため、経緯をADR、Release notes、監視workflowから追える状態を維持する必要がある。
- Linux利用者へartifactを提供できないが、現行のWindows優先運用と一致する。
- ターゲット別`cargo tree`確認はCI時間を少し増やすが、配布対象への依存混入を早期に検出できる。

## 代替案

Dependabot alertを上流修正まで開いたままにする。

不採用理由:

- Windows/macOS配布物には含まれず、実際の配布リスクとalert状態が一致しない。
- 既存の週次監視が上流修正可能状態を検出できるため、Open alertだけに追跡責務を持たせる必要がない。

`glib` 0.20.0をCargo patchで強制する。

不採用理由:

- `gtk` 0.18系の型・ABI互換性を保証できず、Linux依存グラフを破壊する。

## セキュリティ

- リスク受容はWindows/macOS配布物に限定し、Linuxでの脆弱性を解消済みとは扱わない。
- Release workflowとCargo依存グラフの両方を検査する。
- 監視workflowは`contents: read`だけを使用し、アプリ実行時の外部通信やTauri権限を追加しない。
- alertのdismiss理由と対象OSを公開資料に残す。

## 破綻シナリオ

- Linux artifactを追加したのに、dismiss済みalertを理由に安全と誤認する。
- Tauri更新でWindows/macOS側へ`glib`が混入しても検知しない。
- 上流で修正可能になった後も0.18系を固定し続ける。
- Release notesからLinux非対応とリスク受容の説明が消える。

## 再審査条件

- Tauri/GTK系が`glib` 0.20.0以上へ更新可能になる。
- Linux artifactまたはLinuxサポートを追加する。
- Windows/macOSの依存グラフに`glib`が現れる。
- advisoryの影響範囲または深刻度が変更される。

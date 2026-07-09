# 016: glib advisory監視を自動化する

GitHub Issue: #22

## 目的

Tauri経由で入っている `glib` advisoryについて、上流依存が修正可能になった時点で見落とさず依存更新PRへ進められる状態にする。

## 背景

GitHub Dependabot alert #1 は `glib` `>= 0.15.0, < 0.20.0` を対象とし、修正済みバージョンは `0.20.0` 以上である。

現時点の依存経路は、Tauri 2.11.5からLinux向けGTK/WebKit系依存を通じたものであり、`gtk 0.18.2` が `glib ^0.18` を要求している。

```text
tauri 2.11.5 -> gtk 0.18.2 -> glib 0.18.5
```

`cargo update --manifest-path src-tauri/Cargo.toml -p glib --precise 0.20.0` は `gtk = ^0.18` の制約で失敗するため、互換性を無視した `[patch]` や強制上書きは行わない。

## スコープ

- `glib` advisoryがまだ上流制約でブロックされているかを確認するスクリプトを追加する。
- 週次および手動実行のGitHub Actionsで再評価できるようにする。
- 修正可能になった場合はworkflowを失敗させ、Issue #22の依存更新PR作成を促す。
- 監視は開発・運用時の通信として扱い、アプリ実行時の外部通信は追加しない。

## スコープ外

- `glib` 0.20.0以上への強制更新。
- Tauri、GTK、WebKit系依存の互換性を無視したCargo patch。
- Linux artifactの配布追加。
- Dependabot alert #1のクローズ。

## 実装方針

- `scripts/check-glib-advisory.mjs` は `Cargo.lock` を一時的に最新化し、Cargo metadataをJSONとして解析する。
- 判定後は `Cargo.lock` を必ず元に戻し、ローカルやCIの作業ツリーを汚さない。
- 脆弱対象の `glib` が残り、かつ `gtk 0.18.2` / `glib ^0.18` による既知ブロックであれば成功扱いにする。
- 脆弱対象の `glib` が消えた場合、または `glib 0.20.0` 指定が解決できる場合は失敗扱いにする。
- 失敗は「壊れた」ではなく「依存更新PRへ進める合図」として扱う。

## 設計レビュー

### データモデル

アプリのドメインデータ、SQLiteスキーマ、Repository、Use Caseは変更しない。変更対象は依存関係監視の運用境界のみである。

### トランザクション境界

- 監視workflow: Cargo resolverで最新依存状態を一時評価する境界。
- 依存更新PR: `Cargo.lock` と必要な依存宣言を実際に更新する境界。
- Release公開判断: 未解決alertの影響範囲をRelease notesへ反映する境界。

### セキュリティ

- Advisoryを隠さず、Release notesとIssueで影響範囲を追跡する。
- 監視workflowの権限は `contents: read` のみとする。
- アプリ実行時の外部通信やTauri権限は追加しない。
- Linux artifactを追加する場合は、このIssueを解消するまでRelease対象に含めない。

### スケール

週1回のCargo resolver確認だけなので、CI負荷は限定的である。DependabotやCargo index取得は開発・運用時通信として扱う。

## トレードオフ

- 監視workflowを失敗させる方式はGitHub Actions上で気づきやすいが、成功/失敗の意味を運用資料で説明する必要がある。
- Issueコメントを自動投稿する方式は気づきやすいが、`issues: write` 権限が必要になるため今回は採用しない。
- 手動確認だけにすると権限追加は不要だが、上流修正を見落とす可能性が高い。

## 代替案

Dependabot alertだけを見て運用する。

不採用理由:

- Alertが開いたままの状態では「上流制約でまだ更新不能」なのか「更新可能になったのに未対応」なのかが分かりにくい。

`cargo audit` をCIへ追加する。

不採用理由:

- 現在は既知の未解消alertがあるため、通常のPRチェックを常時失敗させやすい。Issue #22の再評価には、今回の専用監視の方が意図を明確にできる。

## 破綻シナリオ

- 上流依存が修正可能になってもIssue #22が放置される。
- 監視workflowの失敗を通常のビルド失敗と誤解し、依存更新PRへ進まない。
- Linux artifactを追加したのに、glib advisoryをRelease notesへ記載しない。
- 強制patchでCargo resolverを壊し、TauriのLinux依存が不整合になる。

## 受け入れ条件

- 手元で `npm run check:glib-advisory` を実行できる。
- GitHub Actionsから手動または週次でglib advisoryを再評価できる。
- workflow権限が `contents: read` に限定されている。
- `Cargo.lock` を変更せずに監視できる。
- Issue #22は、上流依存が修正可能になるまで継続追跡として残る。

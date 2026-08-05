# Windows headless Chrome起動安定化レビュー

## 目的

Windows runner上の `perf:ui` で、Chrome DevTools endpointの公開が10秒以内に完了しない場合があるため、CI計測の起動待機を安定化する。

## 設計判断

- `waitForChromeWebSocket` の既定待機時間を30秒に延長する。
- `HEADLESS_CHROME_STARTUP_TIMEOUT_MS` でCI環境ごとに待機時間を調整可能にする。
- Chromeプロセスが先に終了した場合は、exit codeとsignalを含めて失敗理由を明確化する。
- アプリ本体、Tauri、Repository境界、DB、外部通信仕様は変更しない。

## 代替案

- Windows計測ジョブだけリトライする。
  - 一時的な揺れには強いが、失敗理由の切り分けが遅れるため今回は採用しない。

## トレードオフ

- Chrome起動失敗時の検知が最大30秒まで遅くなる。
- 一方で、Windows runnerの起動ばらつきによる偽陰性を減らせる。

## セキュリティ

- 新しい秘密情報、権限、外部通信は追加しない。
- DevTools接続は従来通りローカル `127.0.0.1` の一時ポートに限定する。

## 危険ケース

- Chrome本体が存在しない場合は従来通り失敗する。
- Chromeが即終了する場合は、今回の変更で失敗理由を明示する。
- 実際のUI性能劣化を待機時間延長で隠さないよう、計測開始後の閾値は変更しない。

## 確認

- `git diff --check`
- `npm run build`

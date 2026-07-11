# ADR 0004: パブリック配布とライセンス

## 状態

採用。

## 文脈

TaskTimerを外部の利用者がGitHubから入手できるようにする。公開リポジトリでは、利用許諾、配布物、問い合わせ先、セキュリティ報告先が曖昧だと、利用者と保守者の双方にリスクが残る。

この判断はアプリ実行時の仕様ではなく、GitHub上の配布・運用方針である。アプリ実行時の外部通信禁止、ローカル保存、ユーザー内容をログに出さない方針は変更しない。

## 決定

- ライセンスはMIT Licenseにする。
- 外部利用者の主な入手経路はGitHub Releasesにする。
- Releaseは `app-v*` タグ、または手動実行のGitHub Actionsからドラフトとして作成する。
- v0.1.0の主配布対象はWindowsにする。
- WindowsはNSISインストーラーを配布する。
- macOSはApple署名・公証準備が完了したReleaseでのみDMGを配布する。
- 自動更新artifactはMVPでは作成しない。
- macOS artifactを配布する場合はDeveloper ID署名とApple公証を行う。詳細は [Issue 014](../issues/014-release-macos-signing-notarization.md) に記録する。
- Windowsコード署名はv0.1.xでは導入せず、OS警告の可能性を既知制限として明記する。詳細は [ADR 0005](0005-windows-code-signing-policy.md) に記録する。

## 設計理由

- MIT Licenseは利用、複製、改変、再配布の許諾が明確で、外部利用者や貢献者が扱いやすい。
- GitHub Releasesは、ソースコード、タグ、リリースノート、配布ファイルを同じ場所で追跡できる。
- Releaseをドラフトにすることで、ビルド成果物、リリースノート、手動確認結果を公開前に確認できる。
- 自動更新をMVPに含めないことで、アプリ実行時の外部通信禁止方針と衝突しない。
- Windowsを先行することで、Apple Developer Program準備を待たずに主利用環境での検証と配布を進められる。

## トレードオフ

- MIT Licenseは再配布や商用利用も許可するため、利用制限を細かく管理できない。
- GitHub Releases中心の配布は、ストア配布よりも企業配布に弱い。
- macOS利用者向けの正式配布は遅れる。
- macOS署名・公証にはApple Developer ProgramとSecrets運用が必要になる。
- Windowsコード署名はv0.1.xでは未導入のため、Windows SmartScreenで警告が出る可能性がある。
- ドラフトRelease運用は、公開前の手動確認が必要な分だけ手間が増える。

## 代替案

- Apache-2.0を採用する。特許許諾が明確になるが、MITよりライセンス文と運用説明が重くなる。
- All rights reservedのままバイナリだけ配布する。再配布や貢献の権利が曖昧になり、外部利用者が扱いにくい。
- GitHub Releasesではなく各OSのストアで配布する。利用者の信頼性は上がるが、MVPでは審査、署名、運用準備が増える。

## 危険ケース

- ReleaseにローカルDB、ログ、秘密情報、個人データを添付してしまう。
- リリースノートに、実タスク名やメモ本文などのユーザー内容を書いてしまう。
- Windows先行ReleaseなのにmacOS artifact提供済みのように記載し、利用者が入手可否を誤解する。
- macOS artifactを配布する場合に、公証失敗またはWindows署名警告を既知制限に書かず、利用者がインストール可否を判断できない。
- Windows署名を導入するときに、証明書、秘密鍵、パスワード、Azure認証情報をGitHub本文やActionsログへ出してしまう。
- 自動更新を追加して、アプリ実行時の外部通信禁止方針を破る。
- ライセンス変更をドキュメントへ反映せず、READMEとLICENSEが矛盾する。

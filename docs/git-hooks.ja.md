# Git フック統合ガイド

[English](git-hooks.md) | [日本語](git-hooks.ja.md)

簡潔なインストール手順は [Git フックとの連携](../README.ja.md#git-フックとの連携)を参照してください。 <!-- rumdl-disable-line MD089 -->
これらは利用者向けの統合です。このリポジトリ自体の開発用フックについては
[貢献ガイド](../CONTRIBUTING.ja.md#git-フック)を参照してください。 <!-- rumdl-disable-line MD089 -->

## pre-commit のセットアップと動作

[pre-commit](https://pre-commit.com/) をインストールし、`.pre-commit-hooks.yaml` を含むリビジョンを使用してください。
`commit-msg` ステージを明示的にインストールしてください。単に `pre-commit install` を実行すると、
代わりに `pre-commit` ステージがインストールされます。
[README の手順](../README.ja.md#pre-commit)には設定とインストールの両方が示されています。 <!-- rumdl-disable-line MD089 -->

初回実行時、pre-commit は Rust 環境を作成して CLI をビルドします。
`cargo --version` が成功すればシステムのツールチェーンを使用し、
そうでなければ rustup を介して Rust をブートストラップし、デフォルトで stable ツールチェーンをインストールします。
ブートストラップ時に rustup や Rust のコンポーネントがダウンロードされる場合があります。
また、キャッシュされていない依存関係のために Cargo がネットワークアクセスを必要とする場合もあります。
オフライン環境や制限された環境では、ツールチェーンと依存関係を用意するか、これらのダウンロードを許可する必要があります。
Rust のセットアップは `cargo install --bins` を `--locked` なしで実行するため、
Cargo はリポジトリの `Cargo.lock` に従わずに依存関係を解決することがあります。

フックは成功した場合も診断を出力します。警告を失敗にするには、引数を上書きします。

```yaml
      - id: commit-msg-lint
        args: [--strict, --edit]
```

`args` を上書きする場合は `--edit` を含めてください。マニフェストがそこで `--edit` を指定しており、
pre-commit がメッセージのファイル名を末尾に追加するためです。
例えば、既存のメッセージファイルを使って設定済みフックをテストできます。

```sh
pre-commit run commit-msg-lint --hook-stage commit-msg --commit-msg-filename message.txt
```

フックが実行されない場合は、`commit-msg` ステージがインストールされ、Git の操作がそれを呼び出すことを確認してください。
`--no-verify` はフックをバイパスします。
セットアップに失敗した場合は、上記のツールチェーンとネットワークの要件を確認してください。
手動で設定したフックでコマンドが見つからない場合は、
インストールされていてフックの `PATH` から利用できることを確認してください。

## 設定ベースのフック

[README の手順](../README.ja.md#設定によるフック)ではローカルフックを登録します。 <!-- rumdl-disable-line MD089 -->
`git hook list --show-scope commit-msg` で確認できます。
試行を削除するには、指定したセクションに保持したい他の設定がないことを確認したうえで、リポジトリから次を実行します。

```sh
git config --local --remove-section hook.commit-msg-lint
```

strict モードにするには、代わりにコマンドを `commit-msg-lint --strict --edit` に設定します。既存のフックスクリプトでは
`commit-msg-lint --strict --edit "$1"` を使用します。設定の詳細は
[Gitフックのドキュメント](https://git-scm.com/docs/git-hook)を参照してください。 <!-- rumdl-disable-line MD089 -->

## Git のクリーンアップとの整合

フックは Git の最終クリーンアップ前にメッセージを受け取ります。CLI のフィルタリングは Git のクリーンアップの近似であり、
その再現ではありません。内容を保持する必要がある場合は、Git のコミットオプションとリンターの設定を一致させてください。
制御方法、プレフィックスの検索、それらの制限、およびメッセージをそのままコミットしながら両方のフィルタリング操作を
無効にする例については、[CLI のフィルタリングリファレンス](cli.ja.md#メッセージのフィルタリング)を正とします。 <!-- rumdl-disable-line MD089 -->
Windows のフックの動作と Git for Windows の CRLF による scissors の動作は検証されていません。

# commit-msg-lint

[English](README.md)

コミットメッセージ、または選択した Git コミットに保存されたメッセージを Conventional Commit の
Rule に照らして検査する Rust CLI です。制限値と重大度を設定できます。インストールせずに
[ブラウザーデモ](https://sgryjp.github.io/commit-msg-lint/)を試せます。 <!-- rumdl-disable-line MD089 -->
デモは CLI のコメント・scissors フィルタリングを行わず、ブラウザー内で検査します。
メッセージと設定はサーバーに送信されません。

## 設計方針

- 文字数ではなく、東アジア文字幅を使って表示幅を測定します。ターミナルやフォントによって
  表示が異なる場合はありますが、コミットログの表示をよりよく反映できます
  （[ADR](docs/adr/0001-display-column-measurement.ja.md)）。
- 推奨する制限と必須の制限を警告とエラーに分け、エージェントが意味を保つための裁量を残します。
- エディターや verbose メッセージに対しては既定で Git の cleanup を近似します。
  CLI が検査する内容は Git が保持する内容と異なる場合があります
  （[ADR](docs/adr/0002-default-message-filtering.ja.md)）。

## インストールとメッセージの検査

Rust ツールチェーンをインストール済みのチェックアウトから CLI をビルド・インストールします。

```sh
cargo install --path .
printf 'feat: add parser\n' | commit-msg-lint
commit-msg-lint --edit .git/COMMIT_EDITMSG
commit-msg-lint --commits HEAD
```

例えば `printf 'feat:subject\n' | commit-msg-lint` は次を出力します。

```text
<stdin>:1: error[header-format] header does not match the Conventional Commit format
```

[診断と終了コード](docs/cli.ja.md#診断と終了コード)を参照してください。 <!-- rumdl-disable-line MD089 -->

## Git フックとの連携

CLI はフックをインストールしません。また、すべてのコミット経路で `commit-msg` が実行される
わけではありません。以下の利用者向け手順は、このリポジトリの
[開発用フック](CONTRIBUTING.ja.md#git-フック)とは別です。 <!-- rumdl-disable-line MD089 -->

### pre-commit

`.pre-commit-hooks.yaml` を含むリリースタグまたはコミットを使い、フックを
`.pre-commit-config.yaml` に追加します。この例ではタグ `0.0.3` を使います。

```yaml
repos:
  - repo: https://github.com/sgryjp/commit-msg-lint
    rev: 0.0.3
    hooks:
      - id: commit-msg-lint
```

`commit-msg` ステージを明示してインストールします。単に `pre-commit install` を実行すると
別のステージがインストールされます。

```sh
pre-commit install --hook-type commit-msg
```

初回実行時は CLI をビルドし、ネットワーク接続が必要になる場合があります。前提条件、オフライン利用、
strict モード、トラブルシューティングは [Git フックガイド](docs/git-hooks.ja.md)を参照してくだ <!-- rumdl-disable-line MD089 -->
さい。

### 既存の commit-msg フック

既存のフックでは、Git のメッセージファイル引数をインストール済みの CLI に渡します。

```sh
commit-msg-lint --edit "$1"
```

### 設定によるフック

Git 2.54 以降で手軽に試すには、まず CLI をインストールし、Git のフックの `PATH` から
利用できるようにします。検査対象のリポジトリで一度だけ実行します。

```sh
git config set --local hook.commit-msg-lint.command 'commit-msg-lint --edit'
git config set --local --append hook.commit-msg-lint.event commit-msg
```

Git は設定したコマンドにメッセージファイル名を渡します。ローカル設定は他のクローンには共有
されません。設定されたフックは既存のフックと並行して実行されるため、既存フックでもこのリンターを
実行している場合、検査が重複することがあります。確認・削除方法は
[Git フックガイド](docs/git-hooks.ja.md#設定ベースのフック)を参照してください。 <!-- rumdl-disable-line MD089 -->

## リンターの設定

設定が見つからない場合は組み込みの既定値が適用されます。Git リポジトリルートの
`commit-msg-lint.jsonc` で任意の設定を上書きできます。`--config <path>` でファイルを
明示的に指定できます。リポジトリルートから、編集可能な既定設定またはインストール済み CLI に
一致するスキーマを生成するには、次を実行します。

```sh
commit-msg-lint --print-default-config > commit-msg-lint.jsonc
commit-msg-lint --print-config-schema > commit-msg-lint.schema.json
```

リダイレクトは出力先ファイルを上書きします。設定の探索とスキーマの互換性については
[設定リファレンス](docs/configuration.ja.md)、[Rule の既定値](docs/configuration.ja.md#組み込み-rule)を参照して <!-- rumdl-disable-line MD089 -->
ください。形式を選択した理由は [JSONC の決定記録](docs/adr/0003-jsonc-configuration.ja.md)を <!-- rumdl-disable-line MD089 -->
参照してください。

## 注意事項と現在の制限

- Lint Rule の警告が出力されてもコマンドが成功する場合があります。`--strict` を指定すると
  失敗します。[終了コード](docs/cli.ja.md#診断と終了コード)を参照してください。 <!-- rumdl-disable-line MD089 -->
- 履歴入力は一つの Git リビジョン、または明示的な二点・三点範囲を受け付け、shallow
  リポジトリを拒否します。[履歴入力ガイド](docs/cli.ja.md#履歴入力)で Git の選択の意味と CI の履歴要件 <!-- rumdl-disable-line MD089 -->
  を説明しています。
- 標準入力と `--edit` では、CLI のコメント・scissors フィルタリングが既定で有効です。
  Git の cleanup を近似するため、検査内容は Git が保持する内容と異なる場合があります。
  [フィルタリング](docs/cli.ja.md#メッセージのフィルタリング)を参照してください。履歴入力はこれらのフィルターを使わず <!-- rumdl-disable-line MD089 -->
  Git の表示用メッセージを検査します。[履歴入力の制限](docs/cli.ja.md#履歴入力)を参照してください。 <!-- rumdl-disable-line MD089 -->
- 既定では、生成されたメッセージの Header と認識されると、`--strict` でもすべての Rule を
  適用しません。判定は生成元ではなくテキストに基づきます。
  [無視されるメッセージ](docs/configuration.ja.md#無視するメッセージ)を参照してください。 <!-- rumdl-disable-line MD089 -->
- カスタム Rule とフックのインストールは現在サポートしていません。

## エージェントからの利用

`commit-msg-lint --help-for-agents` を実行するか、
[エージェント利用ガイド](docs/agent-usage.ja.md)を参照してください。 <!-- rumdl-disable-line MD089 -->

## リファレンス

- [CLI](docs/cli.ja.md): オプション、[履歴入力](docs/cli.ja.md#履歴入力)、
  [フィルタリング](docs/cli.ja.md#メッセージのフィルタリング)、
  [診断と終了コード](docs/cli.ja.md#診断と終了コード)
- [設定](docs/configuration.ja.md): [既定値](docs/configuration.ja.md#組み込み-rule)、
  [無視されるメッセージ](docs/configuration.ja.md#無視するメッセージ)、Rule 設定
- [メッセージ形式](docs/message-format.ja.md): パースと Rule の動作
- [エージェント利用](docs/agent-usage.ja.md)と [Git フック](docs/git-hooks.ja.md) <!-- rumdl-disable-line MD089 -->
- [用語集](GLOSSARY.md)（英語）と[決定記録](docs/adr/) <!-- rumdl-disable-line MD089 -->

## 開発

開発と検証については [Contributing](CONTRIBUTING.ja.md)、リリース履歴については <!-- rumdl-disable-line MD089 -->
[Changelog](CHANGELOG.md)（英語）を参照してください。配布経路、プラットフォームの保証、 <!-- rumdl-disable-line MD089 -->
ライブラリー API のサポートはメンテナーの未決事項です。[TODO](TODO.md)（英語）を参照して <!-- rumdl-disable-line MD089 -->
ください。

## ライセンス

希望に応じて [MIT License](LICENSE-MIT) または
[Apache License, Version 2.0](LICENSE-APACHE) のいずれかで利用できます。

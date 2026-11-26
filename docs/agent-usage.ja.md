# エージェントから `commit-msg-lint` を使う

[English](agent-usage.md) | [日本語](agent-usage.ja.md)

`commit-msg-lint` を使って、完全なコミットメッセージ 1 件または明示的な Git リビジョン範囲で選択したコミットを検査できます。
対応する構文と Git の選択セマンティクスについては、
[CLI の履歴入力ガイド](cli.ja.md#履歴入力)を参照してください。 <!-- rumdl-disable-line MD089 -->
`commit-msg-lint --help-for-agents` を実行すると、このガイドが出力されます。このオプションは、標準入力の読み取り、
Git の呼び出し、設定の検出や読み込み、メッセージフィルタ用の環境変数の読み取りを行わずに、同梱の Markdown を出力します。
他の引数と組み合わせないでください。

## メッセージを渡す

デフォルトでは、リンターは標準入力から EOF まで完全なメッセージを読み取ります。

```sh
printf '%s\n' 'feat: add parser' | commit-msg-lint
```

代わりに `--edit <path>` を使うと、ファイルから完全なメッセージを読み取ります。このモードでは標準入力は無視されます。

```sh
commit-msg-lint --edit message.txt
```

CLI はメッセージを変更しません。診断は標準エラーに出力され、入力元と行番号を含みます。履歴の行番号は Git の出力を指します。
終了コード `0` は lint エラーがないことを意味します。終了コードが `0` でも警告が標準エラーに出力されることがあります。
終了コード `1` は lint エラーが見つかったか、`--strict` により Lint Rule の警告が失敗に昇格したことを意味します。
終了コード `2` は引数、設定、Git、ファイル読み取り、または入力デコードのエラーを意味します。
浅いリポジトリでの履歴入力も `2` を返します。`--strict` は明示的に指定する必要があります。
リンターは呼び出し元に基づいて警告を失敗とすべきか推測しません。

## 完成したメッセージをそのまま検査する

CLI のデフォルトのフィルタリングは編集中のメッセージ向けであり、コメント行と scissors セクションを除外します。
以下の 2 つの環境変数はそれぞれの操作を独立して制御し、どちらもデフォルトでは有効です。

- `COMMIT_MSG_LINT_STRIP_COMMENTS`
- `COMMIT_MSG_LINT_CUT_SCISSORS`

フィルタリングせずに入力全体を検査するには、両方を偽の値に設定します。例えば、この POSIX シェルコマンドは、
完成したメッセージを標準入力に直接渡します。

```sh
printf '%s\n' 'feat: add parser' '' '# retained text' | \
  COMMIT_MSG_LINT_STRIP_COMMENTS=false \
  COMMIT_MSG_LINT_CUT_SCISSORS=false \
  commit-msg-lint
```

これらの変数が影響するのはリンターの入力選択だけです。Git のクリーンアップは設定しません。
Git のクリーンアップモードがどちらかの変数を自動的に変更することも、変数を設定することで Git のクリーンアップが
変わることもありません。特に、`-m` と `-F` に対する Git のデフォルトのクリーンアップでは、
リンターがデフォルトで除外するコメントが保持されることがあります。

## 既存の `commit-msg` フック

既存のフックが `commit-msg-lint` を呼び出す場合、`git commit` コマンドに設定された環境変数は、
フックが明示的に削除または置換しない限り、Git、続いてフックとリンターに継承されます。
これにより、完成したメッセージファイルをそのままコミットする際に、呼び出し元は両者を一致させることができます。

```sh
COMMIT_MSG_LINT_STRIP_COMMENTS=false \
COMMIT_MSG_LINT_CUT_SCISSORS=false \
git -c commit.verbose=false commit --cleanup=verbatim -F message.txt
```

この POSIX シェルの例では、リンターのフィルタリングを無効にすると同時に、それとは別に Git にメッセージをそのまま保持し、
詳細な diff を追加しないように指定しています。一方の指定が他方を意味するわけではありません。
フックは依然としてインストールされ、Git のコミット処理で呼び出される必要があります。
`commit-msg-lint` はフックのインストールや管理を行いません。

すべてのオプション、フィルタリングの詳細、設定の動作については
[CLIリファレンス](cli.ja.md)を参照してください。 <!-- rumdl-disable-line MD089 -->

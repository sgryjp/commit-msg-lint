# commit-msg-lint

コミットメッセージを Conventional Commits と TOML 設定に基づいて検査する Rust 製 CLI。

```sh
cargo install --path .
printf 'feat: add parser\n' | commit-msg-lint
commit-msg-lint --edit .git/COMMIT_EDITMSG
```

`--strict` は warning を終了コード 1 として扱う。`--config <path>` で設定ファイルを
指定できる。省略した場合は Git コマンドを使ってリポジトリのルートを探し、その
`commit-msg-lint.toml` を使用する。設定形式と Rule の詳細は
[要件](docs/requirements/commit-message-linter.md)を参照。

Display Column の測定には `unicode-width` 0.2.2（Unicode 17.0.0）を使用する。
Unicode データの更新時は、この情報と適合テストを更新し、リリースノートに変更を
記載する。

# 寄稿ガイド

このリポジトリでは、Rust 製 CLI と Markdown 文書を検証する。エージェント固有の運用は[AGENTS.
md](AGENTS.md)を正とする。

## 必要なツール

- stable Rust toolchain（`cargo`を含む）
- Python 3
- cjkfmt 0.0.10
- pre-commit 4.4.0 以上、または互換性のある現在の prek

cjkfmt は次のコマンドで導入する。

```sh
cargo install --locked cjkfmt --version 0.0.10
```

hook 実行器には pre-commit または prek のいずれかを導入する。利用中の環境に適した方法で、現在のリ
リースへ更新しておく。

```sh
python -m pip install --user --upgrade pre-commit
# または
python -m pip install --user --upgrade prek
```

## 開発と検証

すべての Rust テストを実行する。

```sh
cargo test --locked
```

fixture-discovery 検査を実行する。

```sh
python scripts/check_fixture_discovery.py
```

Windows では、Python ランチャーを使う場合がある。

```powershell
python scripts/check_fixture_discovery.py
```

## Markdown 文書

追跡対象の`.md`と`.markdown`は、最大 100 幅で、和文と半角英字・数字の境界に半角スペースを要求する。
違反を確認するには、利用している hook 実行器で全ファイルを検査する。

```sh
pre-commit run --all-files
# または
prek run --all-files
```

hook はファイルを変更しない。Markdown を修正するときは、対象ファイルを明示して整形し、差分を確認して
から検査する。自動整形後も診断が残る場合は、Markdown 構造と意味を保ちながら手作業で修正する。

```sh
cjkfmt format --write README.md docs/example.md
cjkfmt check README.md docs/example.md
```

## Git hook

commit 前の Markdown 検査とコミットメッセージ検査を有効化する。

```sh
pre-commit install --hook-type pre-commit --hook-type commit-msg
# または
prek install --hook-type pre-commit --hook-type commit-msg
```

`commit-msg` hook は `cargo run --locked` でこのリポジトリの `commit-msg-lint` を実行する。error が
あるメッセージでは commit が失敗する。warning だけの場合は失敗しない。

## cjkfmt の更新

cjkfmt を更新するときは、hook 定義のリリース、CI で導入するバイナリの版、この文書の版を同じ変更で更
新する。更新前後で全 Markdown の検査を実行する。

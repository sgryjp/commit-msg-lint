# 貢献ガイド

[English](CONTRIBUTING.md)

このリポジトリでは Rust CLI と Markdown 文書を検証します。エージェント固有のワークフローに
ついては、正式なガイドである [AGENTS.md](AGENTS.md)（英語）を参照してください。 <!-- rumdl-disable-line MD089 -->

## 必要なツール

- `cargo` を含む安定版 Rust ツールチェーン
- Python 3
- pre-commit 4.4.0 以降、または互換性のある現行バージョンの prek

フックランナーとして pre-commit または prek のいずれかをインストールします。
環境に適した方法で最新リリースをインストールしてください。

```sh
python -m pip install --user --upgrade pre-commit
# または
python -m pip install --user --upgrade prek
```

## 開発と検証

すべての Rust テストを実行します。

```sh
cargo test --locked --workspace
```

フィクスチャの検出チェックを実行します。

```sh
python scripts/check_fixture_discovery.py
```

Windows では Python ランチャーが必要な場合があります。

```powershell
python scripts/check_fixture_discovery.py
```

### 設定スキーマ

生成された `site/commit-msg-lint.schema.json` は GitHub Pages で公開され、`main` を追跡します。
スキーマ同期のテストを実行するには、次を使います。

```sh
python -B -m unittest discover -s tests -p test_sync_config_schema.py
```

設定の動作を変更した場合は、pre-commit フックでスキーマを再生成して確認します。

```sh
pre-commit run --all-files
# または
prek run --all-files
```

スキーマフックはファイル名を受け取らず常に実行されます。生成されたスキーマが異なる場合、
フックはファイルを更新して失敗し、レビューを促します。ファイルが存在しない、削除されている、
または未追跡の場合も失敗します。未追跡ファイルの内容が期待どおりでも失敗します。
スキーマを確認し、`git add site/commit-msg-lint.schema.json` でステージしてから
チェックを再実行してください。生成に失敗した場合、既存ファイルは保持されます。
報告されたエラーを修正してフックを再実行してください。

## Web デモ

WebAssembly デモをローカルでビルド・プレビューするには、WebAssembly ターゲットと
`wasm-pack` をインストールします。

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked wasm-pack
wasm-pack build web --target web --release --out-dir ../site/pkg
```

`site/index.html` を直接開くのではなく、生成されたファイルを HTTP で配信します。

```sh
cd site
python -m http.server 8000
```

ブラウザーで <http://localhost:8000/> を開きます。生成される `site/pkg/` ディレクトリは
Git で無視されます。

Node.js で Web UI の対象を絞ったテストを実行します（追加パッケージは不要です）。

```sh
node --test tests/site-ui.test.cjs
```

これらのテストは模擬 DOM と WASM API を使います。ビルド済み WebAssembly モジュールも
模擬 DOM とともに検査するには、次を実行します。

```sh
wasm-pack build web --target web --dev --out-dir /tmp/commit-msg-lint-web-pkg
WASM_PKG_DIR=/tmp/commit-msg-lint-web-pkg node --test web/tests/site-wasm.test.cjs
```

実際のブラウザー連携を確認するには、ビルドしたデモをブラウザーでプレビューしてください。

[Pages ワークフロー](.github/workflows/pages.yml)は `main` への push 時にデモをビルド・ <!-- rumdl-disable-line MD089 -->
デプロイします。手動でも実行できます。リポジトリの GitHub Pages ソースは GitHub Actions に
設定する必要があります。

## Markdown 文書

利用者向け文書は、Git とコマンドラインに慣れた開発者を対象に書いてください。
基本概念の不要な説明を避ける一方、このツールの動作、制限、Git との重要な違いは説明してください。
重要な条件を省かず、明確かつ簡潔に書いてください。

利用者向けリファレンスは現在の実装を説明します。CLI の動作、設定、Rule を変更する際は、
関連する文書と例をコードやテストと同じ変更で更新してください。`unicode-width` クレートまたは
Unicode データを更新する際は、文書化されたバージョンと適合性テストを更新し、リリースノートに
変更を記載してください。

`docs/design/` の設計文書は現在の実装を説明します。対象の実装を変更する際は、同じ変更で
設計文書も更新してください。

### 翻訳

英語版と日本語版のある文書は、英語版を正本とします。日本語訳には `.ja.md` の接尾辞を使い、
同期レビューの対象となる文書の対は `docs/translation-sync.json` に一覧があります。既存の日本語のみの
設計文書と調査資料は、対になった翻訳のレビューには含まれません。`docs/agents/` は翻訳の保守対象外であり、
既存の日本語版も保守していません。

貢献者はもう一方の言語を更新せず、どちらかの言語だけを更新できます。そのため翻訳が遅れることが
あります。メンテナーは定期的に文書の対をレビューし、内容を揃えます。手動レビューの手順は
後述します。

### Markdown リント

追跡対象の Markdown ファイルは `.rumdl.toml` を使う rumdl でチェックします。行長ルールの
上限は表示幅 120 桁ですが、一部の長い行は例外です。CJK 間隔ルールは CJK テキストと隣接する
ASCII 英字・数字の間の空白も検査します。リンクやインラインコードのテキストが日本語の場合も
その外側を検査するため、不必要な空白を追加する代わりに `<!-- rumdl-disable-line MD089 -->`
を使ってください。すべてのチェックはフックランナーで実行します。

```sh
pre-commit run --all-files
# または
prek run --all-files
```

Markdown フックはファイルを変更しません。設定スキーマフックは前述のとおり生成ファイルを
更新する場合があります。特定の Markdown ファイルを直接チェックまたは修正するには、
rumdl を別途インストールします。

```sh
cargo install --locked rumdl --version 0.3.5
rumdl check README.md docs/cli.md
rumdl check --fix README.md docs/cli.md
```

フックは独自のコピーをインストールするため、別途のインストールは必須ではありません。
修正後は差分を確認し、Markdown の構造と意味を保ちながら残りの診断に手動で対応してください。

### 翻訳の手動レビュー

メンテナーは任意のタイミングで翻訳をレビューできます。これは CI や貢献時の必須条件では
ありません。レビューには [uv](https://docs.astral.sh/uv/getting-started/installation/)、
Python 3.11 以降、Git、完全な非 shallow 履歴、未追跡ファイルのないクリーンな作業ツリーが必要です。
`uv run` は固定版の Pydantic AI 依存関係をインストールし、適切な Python も取得できます。
無視されたファイルはあっても構いません。各文書の対の基準は以前にレビューしたコミットです。
初期基準は最初の翻訳より前のため、初回レビューには翻訳の追加が含まれます。

翻訳を更新する際は、コマンド、識別子、実際の診断出力を変更しないでください。説明用のコメントは
翻訳できます。日本語訳からは、対応する日本語文書がある場合はそちらへのリンクを優先します。

1. `docs/translation-sync.json` から ID を選び、根拠をプレビューします。

   ```sh
   uv run scripts/check_translation_sync.py review --pair readme --print-prompt
   ```

2. プロバイダーとモデルを明示的に選び、認証情報を設定して実行します。出力の `Reviewing through` に
   表示された SHA を控えます。例えば `OPENAI_API_KEY` を設定し、`YOUR_MODEL` を利用可能な
   モデル名に置き換えて次を実行します。

   ```sh
   uv run scripts/check_translation_sync.py review --pair readme --model openai:YOUR_MODEL
   ```

   `--model anthropic:<model>`（`ANTHROPIC_API_KEY` を使用）と `--model google:<model>`
   （`GOOGLE_API_KEY` を使用）も選択できます。`--model github-copilot:<model>` は Pydantic AI の
   [Copilot 統合](https://ai.pydantic.dev/models/github-copilot/) を使います。認証には
   `GITHUB_COPILOT_API_KEY`、`GITHUB_COPILOT_API_TOKEN`、`COPILOT_GITHUB_TOKEN` のいずれかを
   使用します。`GH_TOKEN` は自動で読み込まれません。トークンとモデルの利用資格、提供者の利用条件、
   組織の方針、課金を事前に確認してください。別のプロバイダーへの自動切り替えはありません。
   他の [Pydantic AI プロバイダー](https://ai.pydantic.dev/models/overview/) も
   `provider:model` 形式で選択できます。追加依存関係が必要な場合は、例えば
   `uv run --with 'pydantic-ai-slim[EXTRA]==2.76.0' scripts/check_translation_sync.py ...` を使います。

   このコマンドは両言語版とその差分、コミットメッセージ、変更されたパスの一覧、変更された Markdown 以外の
   追跡対象テキストファイルの差分と現在の全文を選択したプロバイダーに送ります。他の Markdown 文書は
   パスを列挙しますが、内容と差分は省略します。提供者のデータ方針を含め、事前に共有してよい内容か
   確認してください。関数ツールは登録しないため、モデルは変更されていないソースファイルや省略された
   文書を確認できません。構造化出力ツールはレポートを返すもので、シェルやリポジトリ操作用では
   ありません。省略されたバイナリなどはプロンプトに明示されます。大きな履歴はモデルのコンテキストを
   超える場合があります。対ごとのタイムアウトは既定で 180 秒であり、`--timeout` で変更できます。
   タイムアウト後もリモート側の生成や課金が直ちに止まる保証はありません。対ごとの Pydantic AI の
   モデル要求は 1 回に制限されますが、プロバイダー SDK や通信層が再試行する場合があります。
   `--pair` を繰り返すと複数の対を、`--all` を使うとすべての対を選択できます。対ごとに根拠を再送します。
3. レポートと関連する現在の実装を自分で確認し、必要な修正を行ってコミットします。その後、
   実際にレビューした対についてのみ、確認した SHA を記録します。

   ```sh
   uv run scripts/check_translation_sync.py acknowledge --pair readme --reviewed-through YOUR_REVIEWED_SHA
   ```

   `YOUR_REVIEWED_SHA` を控えた完全な SHA に置き換えます。`--pair` を繰り返すと、同じ SHA で
   複数の対を確認済みにできます。変更された
   `docs/translation-sync.json` を確認してコミットしてください。その SHA 以降のコミットは、
   レビュー開始後に行った修正も含めて次回のレビュー対象に残ります。文書の移動や削除時は
   対応関係の更新が必要です。履歴の欠落や祖先でないチェックポイントはエラーとして報告されます。

モデルの指摘は提案であり、同期の証明ではありません。信頼できないリポジトリの内容にモデルが
誘導されることもあり、形式が正しく指摘がないレポートにも見落としがあり得ます。チェックポイントが
自動的に進むことはありません。エラー、不正または途切れたレポート、不完全と明示されたレポートは
レビューの成功ではありません。不完全なレビューを確認済みにしないでください。オフラインテストは
次のように実行します。

```sh
uv run --python 3.11 --with-requirements scripts/check_translation_sync.py \
  python -B -m unittest discover -s tests -p test_check_translation_sync.py
python -B -m unittest discover -s tests -p test_translation_sync_scope.py
```

## Git フック

スキーマ同期を含む pre-commit チェックとコミットメッセージチェックを有効にします。

```sh
pre-commit install --hook-type pre-commit --hook-type commit-msg
# または
prek install --hook-type pre-commit --hook-type commit-msg
```

`commit-msg` フックは `cargo run --locked` でこのリポジトリの `commit-msg-lint` を実行します。
メッセージにエラーがあるとコミットは失敗します。警告だけでは失敗しません。

## リリース

1. リリース用の変更を準備します。
   - `Cargo.toml` の CLI バージョンと `web/Cargo.toml` の非公開 Web パッケージバージョンを
     更新します。`cargo check --workspace` で `Cargo.lock` を更新し、差分を確認します。
   - `CHANGELOG.md` の該当する Unreleased 項目を日付付きのリリース節に移動します。
     前のリリース以降の利用者に見える追加、修正、動作変更をレビューし、比較リンクを更新します。
     今後の変更のために Unreleased 節を残してください。
2. `cargo test --locked --workspace` と `pre-commit run --all-files`（または
   `prek run --all-files`）を含む前述のチェックを実行します。
   `cargo run --locked -- --version` が新しい CLI バージョンを出力することを確認します。
3. リリース変更を `main` にマージし、リリース対象の正確なコミットで Test ワークフローが
   成功するまで待ちます。リリースワークフローはテストを待ちません。
4. バージョンと一致し、`v` 接頭辞を付けない注釈付きタグを作成して push します。
   以下のバージョンとコミットをリリースバージョンと検証済みコミットに置き換えてください。

   ```sh
   git tag -a 0.0.2 <verified-commit> -m "Release 0.0.2"
   git push origin 0.0.2
   ```

5. [リリースワークフロー](.github/workflows/release.yml)の成功と、GitHub Release が目的のタグを <!-- rumdl-disable-line MD089 -->
   指していることを確認します。ワークフローは `CHANGELOG.md` ではなく GitHub から
   リリースノートを生成するため、変更履歴と照合し、必要に応じて編集します。
   新しいタグを使って pre-commit フックをインストールできることを確認します。

このワークフローはタグの push で実行され、パッケージの公開やビルド成果物の添付は行いません。
ワークフローが利用可能になる前に push されたタグでは、遡って実行されません。GitHub Pages の
Web デモと設定スキーマはリリースタグではなく `main` を追跡します。

## rumdl の更新

rumdl を更新する際は、フックのリリースとこの文書のバージョンを同時に更新します。
更新前後にすべての Markdown ファイルでチェックを実行してください。

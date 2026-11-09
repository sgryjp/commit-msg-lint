# commitlint vs gitlint 比較調査

## はじめに

コミットメッセージの体裁を静的にチェックするツールとして広く使われている二つ、
JavaScript/Node.js 製の **commitlint**
（[conventional-changelog/commitlint](https://github.com/conventional-changelog/commitlint)）と
Python 製の **gitlint**（[jorisroovers/gitlint](https://github.com/jorisroovers/gitlint)）を、
公式ドキュメントサイトおよび GitHub 上のソースコードのみを一次情報源として比較する。
二次情報（ブログ記事やチュートリアル等）は使用せず、すべての事実記述に一次情報源への引用を付す。
確認できなかった事項は推測せず、その旨を明記する。

## 比較の要約

### 1. 言語 / インストール

- **commitlint:** Node.js / `npm install -D @commitlint/cli
  @commitlint/config-conventional`[^cl-getstart]
- **gitlint:** Python / `pip install gitlint`[^gl-index]

### 2. 設定ファイル

- **commitlint:** `commitlint.config.js` など、cosmiconfig 経由で多数の形式に対応する。
  `package.json` の `commitlint` フィールドも利用できる[^cl-config]。
- **gitlint:** `.gitlint`（INI 形式）を使い、セクション単位でルールを設定する[^gl-config]。

### 3. デフォルトルールセット

- **commitlint:** `@commitlint/config-conventional` が Error / Warning で 12 ルールを定義する
  （下記参照）[^cl-conv-index]。
- **gitlint:** 本体の組み込みルール約 20 個（T1〜T8、B1〜B8、M1、I1〜I4）を
  すべて既定で有効にする[^gl-rules]。

### 4. カスタマイズ機構

- **commitlint:** プラグイン（`commitlint-plugin-*`）、ローカルルール関数、
  `RuleConfigSeverity`（Warning=1 / Error=2）を利用できる[^cl-plugins]。
- **gitlint:** `extra-path` で Python クラスをロードするユーザー定義ルールと、
  `contrib` で任意に追加できるコミュニティルールがある。重大度は error / warning に
  区別されず、ルールの結果は pass / violation のみ[^gl-userrules][^gl-contrib][^gl-rules-src]。

### 5. Conventional Commits 準拠度

- **commitlint:** `@commitlint/config-conventional` が Conventional Commits 準拠のパーサー
  （`conventional-changelog-conventionalcommits`）や `type-enum` などを適用する
  [^cl-conv-index]。
- **gitlint:** 既定ルールは Conventional Commits 固有ではなく、一般的なスタイル規則である。
  準拠チェックは `contrib-title-conventional-commits`（CT1）という**オプトインの
  contrib ルール**でのみ提供する[^gl-contrib]。

### 6. Git フック統合

- **commitlint:** 組み込みのフック管理はない。Husky などの外部ツールを使い、
  `commit-msg` フックで `commitlint --edit $1` を実行する[^cl-local]。
- **gitlint:** `gitlint install-hook` / `gitlint uninstall-hook` サブコマンドを
  標準搭載する。既存フックがある場合はインストールを拒否する。また、pre-commit
  framework にも対応する[^gl-hooks]。

### 7. CI 利用パターン

- **commitlint:** GitHub Actions、GitLab CI、CircleCI、Bitbucket、Azure Pipelines、Travis、
  Jenkins X 向けの公式サンプルがある。レンジ指定には `--from` / `--to` / `--last` /
  `--edit` などを使う[^cl-ci][^cl-cli]。
- **gitlint:** 引数なしでは最新コミットを検査する。`--commit`（単一コミット）や
  `--commits`（範囲 / 複数コミット）で範囲を指定でき、stdin 経由のパイプも利用できる
  [^gl-ci][^gl-range]。

### 8. 拡張エコシステム

- **commitlint:** npm の shareable config（`commitlint-config-*`）やプラグイン
  （`commitlint-plugin-*`）を利用できる[^cl-share][^cl-plugins]。
- **gitlint:** 同梱の `contrib` ルール（オプトイン）と `extra-path` によるユーザー定義
  Python ルールがある。npm のような外部パッケージの公式インデックスは、確認した範囲では
  見当たらなかった[^gl-contrib][^gl-userrules]。

### 9. モノレポ対応

- **commitlint:** `guides/monorepo` のような専用ページは現行サイトマップに
  **見当たらなかった**（後述）。一方、`--cwd` や `extends` の相対パス指定など、
  個別対応に使える機能はある[^cl-cli][^cl-share]。
- **gitlint:** モノレポ専用のドキュメントは**見当たらなかった**。

### 10. 編集 / 作成支援

- **commitlint:** `@commitlint/prompt-cli` は unmaintained と明記されている。
  Commitizen アダプタとして `@commitlint/prompt` と `@commitlint/cz-commitlint` がある
  [^cl-prompt]。
- **gitlint:** 公式ドキュメントに、対話的なコミット作成支援ツールは
  **見当たらなかった**。

### 11. バージョン情報

- **commitlint:** `@commitlint/cli` の `package.json` に `"version": "21.2.4"` とあり
  [^cl-cli-pkg]、最新 GitHub リリースタグも `v21.2.4` であった[^cl-releases]。
- **gitlint:** 最新 GitHub リリースタグは `v0.19.1`[^gl-tags]。
  `gitlint-core` の `pyproject.toml` は `dynamic = ["version", "urls"]` とし、
  hatch-vcs でバージョンを管理するため、固定文字列は記載されていない[^gl-pyproject]。

## 各観点の詳細

### 1. 言語/ランタイムとインストール方法

commitlint は Node.js 製で、公式 Getting Started ガイドでは次のコマンドで導入する。

```sh
npm install -D @commitlint/cli @commitlint/config-conventional
```

yarn/pnpm/bun/deno 向けの対応コマンドも同ページに併記されている[^cl-getstart]。

gitlint は Python 製で、トップページに掲載されているインストールコマンドは

```sh
pip install gitlint
```

であり、実行すると `gitlint-core` と `gitlint` がインストールされる旨が明記されている
[^gl-index]。pipx を用いたインストール手順は、閲覧した公式ページの範囲内では確認できなかった。

### 2. 設定ファイル形式と配置場所

commitlint は `@commitlint/cli` が cosmiconfig を用いて次のファイル群を探索する:
`.commitlintrc`, `.commitlintrc.json`, `.commitlintrc.yaml`, `.commitlintrc.yml`,
`.commitlintrc.js`, `.commitlintrc.cjs`, `.commitlintrc.mjs`, `.commitlintrc.ts`,
`.commitlintrc.cts`, `.commitlintrc.mts`, `commitlint.config.js`, `commitlint.config.cjs`,
`commitlint.config.mjs`, `commitlint.config.ts`, `commitlint.config.cts`, `commitlint.config.mts`。
加えて `package.json` 内の `commitlint` フィールドでも設定できる[^cl-config]。

gitlint は `.gitlint` という INI 形式ファイルを使う。`[general]` セクションで
`ignore`・`contrib`・`extra-path` などの全体設定を行い、`[title-max-length]` のように
ルール名（または ID、例: `[T1]`）をセクション名にしてルール単位のオプションを設定する
[^gl-config]。

### 3. 組み込みルールセット / デフォルトプリセット

**commitlint（`@commitlint/config-conventional`）** は GitHub 上の
`@commitlint/config-conventional/src/index.ts` で以下の 12 ルールを定義している
（`RuleConfigSeverity.Error`=2、`.Warning`=1）[^cl-conv-index]:

- `body-leading-blank`: Warning、always。
- `body-max-line-length`: Error、always、100。
- `footer-leading-blank`: Warning、always。
- `footer-max-line-length`: Error、always、100。
- `header-max-length`: Error、always、100。
- `header-trim`: Error、always。
- `subject-case`: Error、never。sentence-case、start-case、pascal-case、upper-case を禁止。
- `subject-empty`: Error、never。
- `subject-full-stop`: Error、never、`.`。
- `type-case`: Error、always、lower-case。
- `type-empty`: Error、never。
- `type-enum`: Error、always。build、chore、ci、docs、feat、fix、perf、refactor、revert、
  style、test。

また `parserPreset: "conventional-changelog-conventionalcommits"` が指定されており、
Conventional Commits 準拠のパーサーが使われる[^cl-conv-index]。commitlint 本体が提供する
全ルールと既定値は `reference/rules.html` に整理されている。`body-case`、`scope-enum`、
`scope-case`、`scope-delimiter-style`、`trailer-exists`、`signed-off-by`、
`breaking-change-exclamation-mark` など、config-conventional が使わないルールも多数ある
[^cl-rules]。

**gitlint** は `gitlint-core/gitlint/rules.py` ですべてのビルトインルールを Python クラス
として実装しており、**全て標準で有効**である[^gl-rules-src]。公式の Built-in Rules
ドキュメントにも「All built-in rules are enabled by default」と明記されている[^gl-rules]。
代表例を挙げる。

- `T1 title-max-length`（既定 72 文字）、`T2 title-trailing-whitespace`,
  `T3 title-trailing-punctuation`, `T4 title-hard-tab`,
  `T5 title-must-not-contain-word`（既定 "WIP"）、`T6 title-leading-whitespace`,
  `T7 title-match-regex`, `T8 title-min-length`（既定 5 文字）。
- `B1 body-max-line-length`（既定 80 文字）、`B2 body-trailing-whitespace`,
  `B3 body-hard-tab`, `B4 body-first-line-empty`,
  `B5 body-min-length`（既定 20 文字）、`B6 body-is-missing`,
  `B7 body-changed-file-mention`, `B8 body-match-regex`。
- `M1 author-valid-email`
- `I1 ignore-by-title`、`I2 ignore-by-body`、`I3 ignore-body-lines`,
  `I4 ignore-by-author-name`（いずれも `ConfigurationRule`。特定条件に合致したコミットの
  他ルールを動的に無効化する）。

以上はソースファイル `rules.py` のクラス定義と、ビルトインルールドキュメントの双方で
確認した[^gl-rules-src][^gl-rules]。

### 4. ルールのカスタマイズ機構

commitlint は JS 関数としてルールを実装するプラグイン方式を採る。プラグインは
`commitlint-plugin-<name>` という npm パッケージ名で `rules` オブジェクトをエクスポートし、
ルール関数は `(parsed, when, value)` のシグネチャを持つ。ローカルプラグインは
`commitlint.config.js` 内に `plugins: [...]` を直接記述し、1 つまで利用できる[^cl-plugins]。
重大度は `RuleConfigSeverity.Warning` (1) と `.Error` (2) の 2 段階がある
（config-conventional の例で確認済み）[^cl-conv-index]。

gitlint はルールを Python クラス（`Rule`／`CommitRule`／`LineRule`／`ConfigurationRule` を
継承）として実装する。`general.extra-path` にディレクトリまたはファイルを指定すると、
ユーザー定義ルールをロードできる[^gl-userrules]。gitlint には commitlint のような
error / warning の重大度区分がなく、`RuleViolation` が発生するかどうかの二値判定のみである。
これは `rules.py` のクラス定義で `RuleViolation` に severity フィールドがないことから
確認した[^gl-rules-src]。

### 5. Conventional Commits 準拠度

commitlint は `@commitlint/config-conventional` を使うと、パーサーに
`conventional-changelog-conventionalcommits` を採用し、`type-enum` で Conventional Commits の
標準的な type 一覧（feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert）を
強制する。これは config-conventional の `index.ts` に明記されている実装である。プリセットを
`extends: ["@commitlint/config-conventional"]` とするだけで有効になり、「デフォルトで
Conventional Commits 寄り」な設計となる[^cl-conv-index]。

gitlint の組み込みデフォルトルールは Conventional Commits 専用ではなく、タイトル長・空白・
句読点などの一般的なスタイル規則にとどまる。Conventional Commits 固有の検査（`type` の
列挙チェックなど）は `contrib-title-conventional-commits`（ID: CT1）という**既定では無効の
contrib ルール**として提供される。`.gitlint` の `[general]` セクションで
`contrib = contrib-title-conventional-commits,CC1` のように明示して有効化する必要がある
[^gl-contrib]。したがって gitlint は、汎用スタイルチェッカーに Conventional Commits 対応を
オプトインで追加する設計である。一方 commitlint は、Conventional Commits 準拠を既定の
推奨設定として提供する。この対比は一次資料から確認できる。

### 6. Git フック統合

commitlint には Git フックを自動設置する組み込みコマンドはない。公式ガイドでは Husky
を使う手順を案内している。`npm install --save-dev husky`、`npx husky init` を実行し、
`.husky/commit-msg` に `npx --no -- commitlint --edit $1` を書き込む[^cl-local]。

gitlint は `gitlint install-hook` / `gitlint uninstall-hook` という**専用サブコマンド**を
標準搭載し、外部ツールなしで `commit-msg` フックを導入・削除できる。既存フックがある場合は
インストールを拒否する[^gl-hooks]。また、[pre-commit](https://pre-commit.com) framework の
フックとしても利用できる。`.pre-commit-config.yaml` に
`repo: https://github.com/jorisroovers/gitlint` の `gitlint` フックを追加し、
`pre-commit install --hook-type commit-msg` を実行する[^gl-hooks]。

### 7. CI での利用パターン

commitlint の公式 CI ガイドには GitHub Actions、Travis、CircleCI、GitLab CI、Jenkins X、
Bitbucket、Azure Pipelines 向けの設定例がある。GitHub Actions の例では push イベントで
`npx commitlint --last --verbose`、pull_request イベントで
`npx commitlint --from ${{ github.event.pull_request.base.sha }} --to
${{ github.event.pull_request.head.sha }} --verbose` を使う[^cl-ci]。CLI リファレンスには
`--from`／`--to`／`--last`／`--edit`／`--env`／`--from-last-tag` などが列挙されている
[^cl-cli]。

gitlint の CI ガイドには、追加パラメータなしで実行すると最新コミットを検査する旨と、
gitlint 自身の CI 設定（`.github/workflows/ci.yml`）へのリンクがある。また、
`git log -1 --pretty=%B | gitlint` のように stdin 経由でも検査できる[^gl-ci]。
複数コミットや範囲は `--commit <ref>`（単一コミット）または
`--commits <range or comma-separated list>`（範囲または複数コミット）で指定する。
`git rev-list` が解釈できる範囲式を利用できる[^gl-range]。commitlint の `--from` / `--to`
に相当する範囲指定として、gitlint の `--commits "A..B"` が機能的に対応する。

### 8. 拡張エコシステム

commitlint は npm 上の shareable config（パッケージ名 `commitlint-config-*`、スコープ付き
なら `@scope/commitlint-config`）を `extends` 配列で読み込める。設定は再帰的に適用され、
shareable config が別の shareable config を extend することもできる[^cl-share]。プラグインは
`commitlint-plugin-*` という命名の npm パッケージとして配布され、`peerDependencies` に
`@commitlint/lint` を指定することが推奨されている[^cl-plugins]。

gitlint は `contrib` ルール（本体に同梱され、オプトインで有効化するルール群: CT1、
CC1、CC2、CC3）を提供する。コミュニティで共通に使われるが、既定では強制しないルールを
集約する設計である[^gl-contrib]。別に `extra-path` を使うユーザー定義 Python ルールもある。
閲覧した公式ドキュメントの範囲では、npm のような外部配布ルール集の公式インデックスは
見当たらなかった。

### 9. モノレポ / 複数設定対応

commitlint の現行サイト（commitlint.js.org）の sitemap / ページ一覧
（VitePress の `__VP_HASH_MAP__`）には、`guides_ai-agents.md`、`guides_ci-setup.md`、
`guides_getting-started.md`、`guides_local-setup.md`、`guides_use-prompt.md`、
`concepts_commit-conventions.md`、`concepts_shareable-config.md` などがある。一方、
`guides_monorepo.md` 相当の専用ページは一覧に含まれず、
`https://commitlint.js.org/guides/monorepo.html` と
`https://commitlint.js.org/concepts/monorepo.html` への直接アクセスはいずれも 404 だった
[^cl-404]。したがって、現行公式サイトに「モノレポ」と銘打った専用ページは確認できない。
ただし、CLI の `--cwd` や `extends` の相対パス指定（`./example` 形式）など、個別に
設定ファイルを切り替える基盤機能は CLI リファレンスと shareable-config ページで確認できる
[^cl-cli][^cl-share]。

gitlint の公式ドキュメント（`jorisroovers.com/gitlint/latest/`）にも、閲覧した範囲では
モノレポ対応を明示するページは見当たらなかった。対象範囲は index、configuration、
commit_hooks、ci、rules/builtin_rules、rules/contrib_rules、rules/user_defined_rules、
linting_specific_commits、configuration/general_options である。`--target` で対象の Git
リポジトリパスを指定できるが[^gl-target]、これはモノレポ対応を謳った機能ではない。

### 10. コミットメッセージ編集・作成支援

commitlint は `@commitlint/prompt-cli` という対話的なコミット作成ツールを提供するが、
公式ガイドには「Prompt is currently unmaintained — Some things might not work as expected」
という警告がある[^cl-prompt]。代替として Commitizen 向けアダプタが 2 種類ある:
`@commitlint/prompt`（`@commitlint/prompt-cli` と同様の対話操作）と
`@commitlint/cz-commitlint`（`cz-conventional-changelog` に着想を得た、より現代的な方式）
[^cl-prompt]。

gitlint の公式ドキュメント群（index ページの Features 一覧を含む）を確認した範囲では、
対話的なコミット作成・編集を支援する機能やツールへの言及は見当たらなかった。
Features 一覧に挙がるのは「commit message hook」「CI」「community contributed rules」、
「user-defined rules」「sane defaults」「easily configurable」「full unicode support」、
「production-ready」であり、対話的プロンプト機能は含まれていない[^gl-index]。

### 11. 成熟度（バージョン等、一次資料に明記されたもののみ）

commitlint は GitHub 上の `@commitlint/cli/package.json` に
`"version": "21.2.4"` と記載されており[^cl-cli-pkg]、GitHub Releases の最新タグも
`v21.2.4` であった[^cl-releases]。CLI リファレンスページの `--help` 出力も、先頭行は
`@commitlint/cli@21.2.4 - Lint your commit messages` となっている[^cl-cli]。

gitlint の GitHub リポジトリでは、ルートの `pyproject.toml` に
`dynamic = ["version", "dependencies", "urls"]`、`gitlint-core/pyproject.toml` に
`dynamic = ["version", "urls"]` とある。hatch-vcs が git タグからバージョンを動的に
決めるため、リポジトリ内に固定のバージョン文字列は見当たらなかった[^gl-pyproject]。
GitHub Releases / Tags の一覧では、最新タグが `v0.19.1` であることを確認した[^gl-tags]。

両者について、「現在何年維持されているか」「ダウンロード数」などの定量的な成熟度指標は、
閲覧した一次資料の範囲内で直接確認できなかった。そのため、本調査では言及を避けた。

## Sources

- https://github.com/conventional-changelog/commitlint/tree/master/@commitlint/config-conventional
  — `src/index.ts` のデフォルトルール 12 個、`parserPreset`、`prompt.questions` を確認。
- [commitlint.js.org: Reference / Rules](https://commitlint.js.org/reference/rules.html)
  — commitlint 本体の全ルール（`body-case`、`scope-enum` など）と既定値を確認。
- [commitlint.js.org: Reference / CLI](https://commitlint.js.org/reference/cli.html)
  — `--help` 出力と `--from` / `--to` / `--last` / `--edit` / `--default-config` などの
  オプション、バージョン `21.2.4` の記載を確認。
- [commitlint configuration reference](https://commitlint.js.org/reference/configuration.html)
  — cosmiconfig の設定ファイル探索一覧、`extends`、`parserPreset`、
  shareable config の例を確認。
- [commitlint.js.org: Guide / Local setup](https://commitlint.js.org/guides/local-setup.html)
  — Husky を使う `commit-msg` フックの設定手順を確認。
- [commitlint.js.org: Guide / CI setup](https://commitlint.js.org/guides/ci-setup.html)
  — GitHub Actions、Travis、CircleCI、GitLab CI、Jenkins X、Bitbucket、
  Azure Pipelines の設定例を確認。
- [commitlint Getting started guide](https://commitlint.js.org/guides/getting-started.html)
  — `npm install -D @commitlint/cli @commitlint/config-conventional` の手順を確認。
- [commitlint.js.org: Guide / Use prompt](https://commitlint.js.org/guides/use-prompt.html)
  — `@commitlint/prompt-cli` が unmaintained であることと、Commitizen アダプタ 2 種を確認。
- [commitlint shareable configuration](https://commitlint.js.org/concepts/shareable-config.html)
  — `commitlint-config-*` の命名規則とスコープパッケージの解決ルールを確認。
- [commitlint.js.org: Reference / Plugins](https://commitlint.js.org/reference/plugins.html)
  — `commitlint-plugin-*` の命名規則、ルール実装方法、ローカルプラグインの制約（1 つまで）を確認。
- [commitlint commit conventions](https://commitlint.js.org/concepts/commit-conventions.html)
  — 複数スコープのサポートと `scope-delimiter-style` ルールを確認。
- [`/guides/monorepo.html`](https://commitlint.js.org/guides/monorepo.html) と
  [`/concepts/monorepo.html`](https://commitlint.js.org/concepts/monorepo.html)
  — どちらも 404 であり、モノレポ専用ページがない根拠とした。
- GitHub API: `repos/conventional-changelog/commitlint/contents/@commitlint/cli/package.json`
  — `"version": "21.2.4"` を確認。
- GitHub API: `repos/conventional-changelog/commitlint/releases`
  — 最新タグ `v21.2.4` を確認。
- [gitlint 公式ドキュメント トップページ](https://jorisroovers.com/gitlint/latest/)
  — `pip install gitlint` と Features 一覧を確認（対話的プロンプト機能の記載なし）。
- [gitlint 公式ドキュメント: Configuration](https://jorisroovers.com/gitlint/latest/configuration/)
  — `.gitlint` の INI 形式、`[general]`、ルール名 / ID での設定、CLI `-c`、環境変数対応を確認。
- [gitlint general options](https://jorisroovers.com/gitlint/latest/configuration/general_options/)
  — `--ignore`、`--target`、`--commit`、`--debug`、`--silent`、`--verbose` などを確認。
- [gitlint commit hook setup](https://jorisroovers.com/gitlint/latest/commit_hooks/)
  — `gitlint install-hook` / `uninstall-hook` と pre-commit framework の連携を確認。
- [gitlint CI guide](https://jorisroovers.com/gitlint/latest/ci/)
  — 引数なし実行時の挙動と stdin 経由でのリント方法を確認。
- [gitlint specific commits](https://jorisroovers.com/gitlint/latest/linting_specific_commits/)
  — `--commit` / `--commits` による範囲指定方法を確認。
- [gitlint built-in rules](https://jorisroovers.com/gitlint/latest/rules/builtin_rules/)
  — T1〜T8、B1〜B5 などのルール名・ID・既定値と、全ルールが既定で有効との記載を確認。
- [gitlint contrib rules](https://jorisroovers.com/gitlint/latest/rules/contrib_rules/)
  — `contrib-title-conventional-commits` (CT1) を含む 4 つの contrib ルールと有効化方法を確認。
- [gitlint user-defined rules](https://jorisroovers.com/gitlint/latest/rules/user_defined_rules/)
  — `extra-path` でユーザー定義 Python ルールをロードする方法を確認。
- [rules.py](https://github.com/jorisroovers/gitlint/blob/main/gitlint-core/gitlint/rules.py)
  — SHA: f49cb1c2f9da30e6991ce5c86f0f2927662afbfe。`Rule` / `CommitRule` / `LineRule` /
  `ConfigurationRule` の継承構造、組み込みルールの実装、`RuleViolation` に severity
  フィールドがないことを確認。
- [pyproject.toml](https://github.com/jorisroovers/gitlint/blob/main/gitlint-core/pyproject.toml)
  — `dynamic = ["version", "urls"]` と hatch-vcs によるバージョン管理を確認。
- GitHub API: `repos/jorisroovers/gitlint/tags` — 最新タグ `v0.19.1` を確認。

## 確認できなかった事項（ギャップ）

- commitlint・gitlint いずれの公式サイトでも、閲覧した範囲内ではモノレポ対応を正面から
  扱う専用ページは見つからなかった。commitlint 側は過去に該当ページが存在した可能性が
  あるが、現行サイトマップには含まれていない。
- gitlint の公式ドキュメントと README を確認した範囲では、commitlint の
  `@commitlint/prompt-cli` や Commitizen アダプタに相当する対話的なコミット作成支援は
  見当たらなかった。「存在しない」とは一次資料から直接証明できないため、
  「見当たらなかった」という表現にとどめた。
- gitlint の正確なリリースバージョン文字列は `pyproject.toml` が `dynamic` 指定のため、
  リポジトリのソースから固定のバージョン番号を直接読み取れなかった。GitHub のタグ名
  （`v0.19.1`）を代用根拠とした。
- pipx などの代替インストール手段について、gitlint 公式トップページで確認できたのは
  `pip install gitlint` のみである。pipx 向けの公式記載は、閲覧した範囲にはなかった。
- npm 上の shareable config やプラグインの具体的な一覧について、commitlint 公式サイトには
  命名規則と仕組みの説明がある。公式ドキュメント内に主要な項目を一覧化したページ
  （`reference/community-projects.md` など）があることまでは把握したが、今回は一覧の内容を
  詳しく確認していない。

[^cl-getstart]: https://commitlint.js.org/guides/getting-started.html
[^cl-config]: https://commitlint.js.org/reference/configuration.html
[^cl-conv-index]: https://github.com/conventional-changelog/commitlint/tree/master/%40commitlint
    Source file: `config-conventional/src/index.ts`.
[^cl-rules]: https://commitlint.js.org/reference/rules.html
[^cl-plugins]: https://commitlint.js.org/reference/plugins.html
[^cl-local]: https://commitlint.js.org/guides/local-setup.html
[^cl-ci]: https://commitlint.js.org/guides/ci-setup.html
[^cl-cli]: https://commitlint.js.org/reference/cli.html
[^cl-share]: https://commitlint.js.org/concepts/shareable-config.html
[^cl-prompt]: https://commitlint.js.org/guides/use-prompt.html
[^cl-cli-pkg]: [commitlint GitHub repository](https://github.com/conventional-changelog/commitlint)
    (GitHub API: `@commitlint/cli/package.json`, `"version": "21.2.4"`)
[^cl-releases]: https://github.com/conventional-changelog/commitlint/releases
[^cl-404]: [`/guides/monorepo.html`](https://commitlint.js.org/guides/monorepo.html) と
    [`/concepts/monorepo.html`](https://commitlint.js.org/concepts/monorepo.html)
    （いずれも 404 確認）
[^gl-index]: https://jorisroovers.com/gitlint/latest/
[^gl-config]: https://jorisroovers.com/gitlint/latest/configuration/
[^gl-rules]: https://jorisroovers.com/gitlint/latest/rules/builtin_rules/
[^gl-rules-src]: https://github.com/jorisroovers/gitlint/blob/main/gitlint-core/gitlint/rules.py
[^gl-contrib]: https://jorisroovers.com/gitlint/latest/rules/contrib_rules/
[^gl-userrules]: https://jorisroovers.com/gitlint/latest/rules/user_defined_rules/
[^gl-hooks]: https://jorisroovers.com/gitlint/latest/commit_hooks/
[^gl-ci]: https://jorisroovers.com/gitlint/latest/ci/
[^gl-range]: https://jorisroovers.com/gitlint/latest/linting_specific_commits/
[^gl-target]: https://jorisroovers.com/gitlint/latest/configuration/general_options/
[^gl-pyproject]: https://github.com/jorisroovers/gitlint/blob/main/gitlint-core/pyproject.toml
[^gl-tags]: https://github.com/jorisroovers/gitlint/tags

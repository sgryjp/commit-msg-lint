# commitlint vs gitlint 比較調査

## はじめに

コミットメッセージの体裁を静的にチェックするツールとして広く使われている二つ、
JavaScript/Node.js 製の **commitlint**（[conventional-changelog/commitlint](https://github.com/conventional-changelog/commitlint)）と
Python 製の **gitlint**（[jorisroovers/gitlint](https://github.com/jorisroovers/gitlint)）を、
公式ドキュメントサイトおよび GitHub 上のソースコードのみを一次情報源として比較する。
二次情報（ブログ記事やチュートリアル等）は使用せず、すべての事実記述に一次情報源への引用を付す。
確認できなかった事項は推測せず、その旨を明記する。

## 比較表

| 観点 | commitlint | gitlint |
|---|---|---|
| 1. 言語/インストール | Node.js / `npm install -D @commitlint/cli @commitlint/config-conventional`[^cl-getstart] | Python / `pip install gitlint`[^gl-index] |
| 2. 設定ファイル | `commitlint.config.js` 等 cosmiconfig 経由で多数の形式に対応、`package.json` の `commitlint` フィールドも可[^cl-config] | `.gitlint`（INI 形式）、セクションでルール単位の設定[^gl-config] |
| 3. デフォルトルールセット | `@commitlint/config-conventional` が 12 ルールを Error/Warning で定義（下記参照）[^cl-conv-index] | gitlint 本体がデフォルトで約20の組み込みルール（T1〜T8, B1〜B8, M1, I1〜I4）を**すべて有効**で提供[^gl-rules] |
| 4. カスタマイズ機構 | プラグイン（`commitlint-plugin-*`）、ローカルルール関数、`RuleConfigSeverity`（Warning=1/Error=2）[^cl-plugins] | `extra-path` で Python クラスをロードするユーザー定義ルール、`contrib` で任意追加できるコミュニティルール。severity（error/warning)の区別は無く、ルールは pass/violation のみ[^gl-userrules][^gl-contrib][^gl-rules-src] |
| 5. Conventional Commits 準拠度 | `@commitlint/config-conventional` がデフォルトで Conventional Commits 準拠のパーサー (`conventional-changelog-conventionalcommits`) と type-enum 等を適用[^cl-conv-index] | デフォルトルールは Conventional Commits 固有ではなく一般的なスタイル規則。Conventional Commits 準拠は `contrib-title-conventional-commits` (CT1) という**オプトインの contrib ルール**でのみ提供[^gl-contrib] |
| 6. Git フック統合 | 組み込みのフック管理無し。Husky 等の外部ツールで `commit-msg` フックに `commitlint --edit $1` を仕込む想定[^cl-local] | `gitlint install-hook` / `gitlint uninstall-hook` というサブコマンドを**標準搭載**。pre-commit framework 対応もあり[^gl-hooks] |
| 7. CI 利用パターン | GitHub Actions・GitLab CI・CircleCI・Bitbucket・Azure Pipelines・Travis・Jenkins X 向けの公式サンプルを掲載、`--from`/`--to`/`--last`/`--edit` などのレンジ指定フラグあり[^cl-ci][^cl-cli] | `gitlint` は引数無しで最新コミットをチェック、`--commit`（単一コミット）・`--commits`（レンジ／複数コミット）で範囲指定可能。stdin 経由のパイプ利用も可[^gl-ci][^gl-range] |
| 8. 拡張エコシステム | npm 上の shareable config（`commitlint-config-*`）、プラグイン（`commitlint-plugin-*`）[^cl-share][^cl-plugins] | `contrib` ルール（gitlint 本体同梱、オプトイン）と `extra-path` によるユーザー定義 Python ルール。npm 的な外部パッケージエコシステムの記載は見当たらない[^gl-contrib][^gl-userrules] |
| 9. モノレポ対応 | 専用の `guides/monorepo` のようなページは現行サイトマップに**見当たらなかった**(後述)。`--cwd` フラグや `extends` の相対パス指定等で個別対応は可能[^cl-cli][^cl-share] | モノレポに関する専用ドキュメントは**見当たらなかった** |
| 10. 編集/作成支援 | `@commitlint/prompt-cli`（現在 unmaintained と明記）、Commitizen アダプタ `@commitlint/prompt` と `@commitlint/cz-commitlint`[^cl-prompt] | 対話的なコミットメッセージ作成支援ツールは公式ドキュメント中に**見当たらなかった** |
| 11. バージョン情報 | `@commitlint/cli` package.json: `"version": "21.2.4"`[^cl-cli-pkg]、最新 GitHub リリースタグ `v21.2.4`[^cl-releases] | 最新 GitHub リリースタグ `v0.19.1`[^gl-tags]、`gitlint-core` の `pyproject.toml` は `dynamic = ["version", "urls"]`（hatch-vcs によるバージョン管理）で固定文字列の記載は無し[^gl-pyproject] |

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

であり、実行すると `gitlint-core` と `gitlint` がインストールされる旨が明記されている[^gl-index]。pipx を用いたインストール手順は、閲覧した公式ページ範囲内では確認できなかった。

### 2. 設定ファイル形式と配置場所

commitlint は `@commitlint/cli` が cosmiconfig を用いて次のファイル群を探索する: `.commitlintrc`, `.commitlintrc.json`, `.commitlintrc.yaml`, `.commitlintrc.yml`, `.commitlintrc.js`, `.commitlintrc.cjs`, `.commitlintrc.mjs`, `.commitlintrc.ts`, `.commitlintrc.cts`, `.commitlintrc.mts`, `commitlint.config.js`, `commitlint.config.cjs`, `commitlint.config.mjs`, `commitlint.config.ts`, `commitlint.config.cts`, `commitlint.config.mts`。加えて `package.json` 内の `commitlint` フィールドでも設定できる[^cl-config]。

gitlint は `.gitlint` という INI 形式ファイルを使う。`[general]` セクションで `ignore`・`contrib`・`extra-path` 等の全体設定を行い、`[title-max-length]` のようにルール名（または ID、例: `[T1]`）をセクション名にしてルール単位のオプションを設定する[^gl-config]。

### 3. 組み込みルールセット / デフォルトプリセット

**commitlint（`@commitlint/config-conventional`）** は GitHub 上の `@commitlint/config-conventional/src/index.ts` で以下の12ルールを定義している（`RuleConfigSeverity.Error`=2, `.Warning`=1）[^cl-conv-index]:

| ルール | severity | condition | value |
|---|---|---|---|
| `body-leading-blank` | Warning | always | - |
| `body-max-line-length` | Error | always | 100 |
| `footer-leading-blank` | Warning | always | - |
| `footer-max-line-length` | Error | always | 100 |
| `header-max-length` | Error | always | 100 |
| `header-trim` | Error | always | - |
| `subject-case` | Error | never | sentence-case, start-case, pascal-case, upper-case を禁止 |
| `subject-empty` | Error | never | - |
| `subject-full-stop` | Error | never | `.` |
| `type-case` | Error | always | lower-case |
| `type-empty` | Error | never | - |
| `type-enum` | Error | always | build, chore, ci, docs, feat, fix, perf, refactor, revert, style, test |

また `parserPreset: "conventional-changelog-conventionalcommits"` が指定されており、Conventional Commits 準拠のパーサーが使われる[^cl-conv-index]。なお commitlint 本体が提供する全ルールの一覧とその既定値は `reference/rules.html` に整理されており、`body-case`, `scope-enum`, `scope-case`, `scope-delimiter-style`, `trailer-exists`, `signed-off-by`, `breaking-change-exclamation-mark` 等、config-conventional が使わないルールも多数存在する[^cl-rules]。

**gitlint** は `gitlint-core/gitlint/rules.py` にてすべてのビルトインルールを Python クラスとして実装しており、**全て標準で有効**である[^gl-rules-src]。公式の Built-in Rules ドキュメントでも「All built-in rules are enabled by default」と明記されている[^gl-rules]。代表的なものを挙げると:

- `T1 title-max-length`（既定72文字）、`T2 title-trailing-whitespace`、`T3 title-trailing-punctuation`、`T4 title-hard-tab`、`T5 title-must-not-contain-word`（既定 "WIP"）、`T6 title-leading-whitespace`、`T7 title-match-regex`、`T8 title-min-length`（既定5文字）
- `B1 body-max-line-length`（既定80文字）、`B2 body-trailing-whitespace`、`B3 body-hard-tab`、`B4 body-first-line-empty`、`B5 body-min-length`（既定20文字）、`B6 body-is-missing`、`B7 body-changed-file-mention`、`B8 body-match-regex`
- `M1 author-valid-email`
- `I1 ignore-by-title`、`I2 ignore-by-body`、`I3 ignore-body-lines`、`I4 ignore-by-author-name`（いずれも `ConfigurationRule`、特定条件に合致したコミットの他ルールを動的に無効化する）

以上はソースファイル `rules.py` のクラス定義およびビルトインルールドキュメントの双方で確認した[^gl-rules-src][^gl-rules]。

### 4. ルールのカスタマイズ機構

commitlint は JS 関数としてルールを実装するプラグイン方式を採る。プラグインは `commitlint-plugin-<name>` という npm パッケージ名で `rules` オブジェクトをエクスポートし、ルール関数は `(parsed, when, value)` のシグネチャを持つ。ローカルプラグイン（`commitlint.config.js` 内に直接 `plugins: [...]` を記述）も1つまで利用できる[^cl-plugins]。severity は `RuleConfigSeverity.Warning`(1) と `.Error`(2) の2段階がある（config-conventional の例で確認済み）[^cl-conv-index]。

gitlint はルールを Python クラス（`Rule`／`CommitRule`／`LineRule`／`ConfigurationRule` を継承）として実装し、`general.extra-path` にディレクトリまたはファイルを指定することで、ユーザー定義ルールをロードできる[^gl-userrules]。gitlint のルールには commitlint のような error/warning の重大度区分はなく、`RuleViolation` が発生するかどうかの二値判定のみである。これは `rules.py` のクラス定義（`RuleViolation` にseverityフィールドが存在しないこと）から確認した[^gl-rules-src]。

### 5. Conventional Commits 準拠度

commitlint は `@commitlint/config-conventional` を使うことで、パーサーに `conventional-changelog-conventionalcommits` を採用し、`type-enum` で Conventional Commits の標準的な type 一覧（feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert）を強制する。これは config-conventional の `index.ts` に明記されている実装であり、プリセットを `extends: ["@commitlint/config-conventional"]` するだけで有効になる、いわば「デフォルトで Conventional Commits 寄り」な設計である[^cl-conv-index]。

gitlint は逆に、組み込みデフォルトルール自体は Conventional Commits 専用ではなく、タイトル長・空白・句読点など一般的なスタイル規則にとどまる。Conventional Commits 固有の検査（`type` の列挙チェック等）は `contrib-title-conventional-commits`（ID: CT1）という**contrib ルール（既定では無効）** として提供されており、`.gitlint` の `[general]` セクションで明示的に `contrib = contrib-title-conventional-commits,CC1` のように有効化しないと適用されない[^gl-contrib]。したがって、gitlint は「汎用スタイルチェッカーに Conventional Commits 対応をオプトインで追加する」設計であり、commitlint は「Conventional Commits 準拠をデフォルトの推奨設定として提供する」設計という対比が一次資料から確認できる。

### 6. Git フック統合

commitlint には git フックを自動設置する組み込みコマンドは無く、公式ガイドでは Husky の利用を前提とした手順が案内されている。具体的には `npm install --save-dev husky` → `npx husky init` → `.husky/commit-msg` に `npx --no -- commitlint --edit $1` を書き込む、という手順である[^cl-local]。

gitlint は `gitlint install-hook` / `gitlint uninstall-hook` という**専用サブコマンド**を標準搭載しており、外部ツールなしで `commit-msg` フックを導入・削除できる（既存フックがある場合はインストールを拒否する仕様）[^gl-hooks]。さらに pre-commit framework（https://pre-commit.com）のフックとしても利用可能で、`.pre-commit-config.yaml` に `repo: https://github.com/jorisroovers/gitlint` の `gitlint` フックを追加し `pre-commit install --hook-type commit-msg` を実行する手順が案内されている[^gl-hooks]。

### 7. CI での利用パターン

commitlint の公式 CI ガイドには GitHub Actions・Travis・CircleCI・GitLab CI・Jenkins X・Bitbucket・Azure Pipelines 向けの設定例がある。GitHub Actions の例では push イベントで `npx commitlint --last --verbose`、pull_request イベントで `npx commitlint --from ${{ github.event.pull_request.base.sha }} --to ${{ github.event.pull_request.head.sha }} --verbose` を使う[^cl-ci]。CLI リファレンスには `--from`／`--to`／`--last`／`--edit`／`--env`／`--from-last-tag` 等のオプションが列挙されている[^cl-cli]。

gitlint の CI ガイドには追加パラメータ無しで実行すると最新コミットをチェックする旨、および gitlint 自身の CI 設定（`.github/workflows/ci.yml`）へのリンクが示されている。また `git log -1 --pretty=%B | gitlint` のように標準入力経由でもチェックできる[^gl-ci]。複数コミットやレンジの指定は `--commit <ref>`（単一コミット）、`--commits <range or comma-separated list>`（レンジまたは複数コミット、`git rev-list` が解釈できる範囲式が利用可能）で行う[^gl-range]。commitlint の `--from`/`--to` に相当する「コミット範囲指定」としては gitlint の `--commits "A..B"` が機能的に対応する。

### 8. 拡張エコシステム

commitlint は npm 上の shareable config（パッケージ名 `commitlint-config-*`、スコープ付きなら `@scope/commitlint-config`）を `extends` 配列で読み込める。これは再帰的に動作し、shareable config がさらに別の shareable config を extend できる[^cl-share]。プラグインは `commitlint-plugin-*` という命名規則の npm パッケージとして配布され、`peerDependencies` に `@commitlint/lint` を指定することが推奨されている[^cl-plugins]。

gitlint は `contrib` ルール（gitlint 本体に同梱され、オプトインで有効化するルール群: CT1, CC1, CC2, CC3）を提供しており、コミュニティで共通に使われるがデフォルトでは強制したくないルールをここに集約する設計である[^gl-contrib]。これとは別に `extra-path` によるユーザー定義 Python ルールの仕組みもあるが、閲覧した公式ドキュメントの範囲では npm のような「外部パッケージとして配布・公開されたルール集」を公式にインデックスする仕組みは見当たらなかった。

### 9. モノレポ / 複数設定対応

commitlint の現行サイト（commitlint.js.org）の sitemap/ページ一覧（VitePress の `__VP_HASH_MAP__`）を確認したところ、`guides_ai-agents.md`, `guides_ci-setup.md`, `guides_getting-started.md`, `guides_local-setup.md`, `guides_use-prompt.md`, `concepts_commit-conventions.md`, `concepts_shareable-config.md` 等は存在するが、モノレポ専用ページ（例: `guides_monorepo.md` 相当）は一覧に含まれておらず、`https://commitlint.js.org/guides/monorepo.html` および `concepts/monorepo.html` への直接アクセスはいずれも404であった[^cl-404]。したがって、現行バージョンの公式サイトには「モノレポ」と銘打った専用ドキュメントページは確認できなかった。ただし CLI の `--cwd` オプションや、`extends` の相対パス指定（`./example` 形式）など、個別プロジェクトごとに設定ファイルを切り替えるための基盤的な機能自体は CLI リファレンスおよび shareable-config ページから確認できる[^cl-cli][^cl-share]。

gitlint の公式ドキュメント（`jorisroovers.com/gitlint/latest/`）内でも、モノレポ対応を明示的に扱うページは閲覧した範囲（index, configuration, commit_hooks, ci, rules/builtin_rules, rules/contrib_rules, rules/user_defined_rules, linting_specific_commits, configuration/general_options）には見当たらなかった。`--target` オプションでリント対象の Git リポジトリパスを指定できる一般機能はあるが[^gl-target]、これはモノレポ対応を謳った機能ではない。

### 10. コミットメッセージ編集・作成支援

commitlint は `@commitlint/prompt-cli` という対話的コミットメッセージ作成ツールを提供しているが、公式ガイドには「Prompt is currently unmaintained — Some things might not work as expected」という警告が明記されている[^cl-prompt]。代替として Commitizen 向けのアダプタが2種類提供されている: `@commitlint/prompt`（`@commitlint/prompt-cli` と同様の対話操作を提供）と `@commitlint/cz-commitlint`（`cz-conventional-changelog` に着想を得た、より現代的な対話方式）[^cl-prompt]。

gitlint については閲覧した公式ドキュメント群（index ページの Features 一覧を含む）の範囲内で、対話的なコミットメッセージ作成・編集を支援する機能やツールへの言及は見当たらなかった。gitlint の Features 一覧に挙がるのは「commit message hook」「CI」「community contributed rules」「user-defined rules」「sane defaults」「easily configurable」「full unicode support」「production-ready」のみであり、対話的プロンプト機能は含まれていない[^gl-index]。

### 11. 成熟度（バージョン等、一次資料に明記されたもののみ）

commitlint は GitHub 上の `@commitlint/cli/package.json` に `"version": "21.2.4"` と記載されており[^cl-cli-pkg]、GitHub Releases の最新タグも `v21.2.4` であった[^cl-releases]。また CLI リファレンスページに掲載されている `--help` 出力の先頭行も `@commitlint/cli@21.2.4 - Lint your commit messages` となっている[^cl-cli]。

gitlint は GitHub リポジトリの `pyproject.toml`（ルート、ワークスペース共通設定）を確認したところ `dynamic = ["version", "dependencies", "urls"]` および `gitlint-core/pyproject.toml` でも `dynamic = ["version", "urls"]` となっており、hatch-vcs によって git タグからバージョンが動的に決定される方式であるため、リポジトリ内に固定のバージョン文字列は見当たらなかった[^gl-pyproject]。GitHub Releases/Tags の一覧からは最新タグが `v0.19.1` であることを確認した[^gl-tags]。

なお、両者とも「現在何年維持されているか」「ダウンロード数」等の定量的な成熟度指標について、閲覧した一次資料の範囲内では直接的な記載を確認できなかったため、本調査ではそれらについての言及は避けた。

## Sources

- [commitlint GitHub: `@commitlint/config-conventional/src/index.ts`](https://github.com/conventional-changelog/commitlint/blob/master/%40commitlint/config-conventional/src/index.ts) — デフォルトルール12個の定義、`parserPreset`、`prompt.questions` の内容を確認。
- [commitlint.js.org: Reference / Rules](https://commitlint.js.org/reference/rules.html) — commitlint本体が提供する全ルール（`body-case`, `scope-enum` 等含む）とその既定値を確認。
- [commitlint.js.org: Reference / CLI](https://commitlint.js.org/reference/cli.html) — `--help` 出力全文、`--from`/`--to`/`--last`/`--edit`/`--default-config` 等のオプションを確認。バージョン `21.2.4` の記載も確認。
- [commitlint.js.org: Reference / Configuration](https://commitlint.js.org/reference/configuration.html) — 設定ファイルの探索対象一覧（cosmiconfig）、`extends`、`parserPreset`、shareable config の例を確認。
- [commitlint.js.org: Guide / Local setup](https://commitlint.js.org/guides/local-setup.html) — Husky を用いた `commit-msg` フック設置手順を確認。
- [commitlint.js.org: Guide / CI setup](https://commitlint.js.org/guides/ci-setup.html) — GitHub Actions / Travis / CircleCI / GitLab CI / Jenkins X / Bitbucket / Azure Pipelines の設定例を確認。
- [commitlint.js.org: Guide / Getting started](https://commitlint.js.org/guides/getting-started.html) — `npm install -D @commitlint/cli @commitlint/config-conventional` のインストール手順を確認。
- [commitlint.js.org: Guide / Use prompt](https://commitlint.js.org/guides/use-prompt.html) — `@commitlint/prompt-cli` が unmaintained である旨、Commitizen アダプタ2種の記載を確認。
- [commitlint.js.org: Concept / Shareable configuration](https://commitlint.js.org/concepts/shareable-config.html) — `commitlint-config-*` の命名規則、スコープパッケージの解決ルールを確認。
- [commitlint.js.org: Reference / Plugins](https://commitlint.js.org/reference/plugins.html) — `commitlint-plugin-*` の命名規則、プラグインのルール実装方法、ローカルプラグインの制約（1つまで）を確認。
- [commitlint.js.org: Concept / Commit conventions](https://commitlint.js.org/concepts/commit-conventions.html) — 複数スコープのサポート、`scope-delimiter-style` ルールへの言及を確認。
- [commitlint.js.org `/guides/monorepo.html`](https://commitlint.js.org/guides/monorepo.html) ・ [`/concepts/monorepo.html`](https://commitlint.js.org/concepts/monorepo.html) — いずれも404であることを確認（モノレポ専用ページ不在の根拠）。
- GitHub API: `repos/conventional-changelog/commitlint/contents/@commitlint/cli/package.json` — `"version": "21.2.4"` を確認。
- GitHub API: `repos/conventional-changelog/commitlint/releases` — 最新タグ `v21.2.4` を確認。
- [gitlint 公式ドキュメント トップページ](https://jorisroovers.com/gitlint/latest/) — `pip install gitlint` のインストール手順、Features 一覧（対話的プロンプト機能の不在を含む）を確認。
- [gitlint 公式ドキュメント: Configuration](https://jorisroovers.com/gitlint/latest/configuration/) — `.gitlint` の INI 形式、`[general]` セクション、ルール名/IDでのセクション指定、CLI `-c` オプション、環境変数対応を確認。
- [gitlint 公式ドキュメント: Configuration / General options](https://jorisroovers.com/gitlint/latest/configuration/general_options/) — `--ignore`, `--target`, `--commit`, `--debug`, `--silent`, `--verbose` 等の一般オプションを確認。
- [gitlint 公式ドキュメント: Commit hook Setup](https://jorisroovers.com/gitlint/latest/commit_hooks/) — `gitlint install-hook`/`uninstall-hook`、pre-commit framework 連携手順を確認。
- [gitlint 公式ドキュメント: Using gitlint in a CI environment](https://jorisroovers.com/gitlint/latest/ci/) — 引数無し実行時の挙動、stdin 経由でのリント方法を確認。
- [gitlint 公式ドキュメント: Linting specific commits](https://jorisroovers.com/gitlint/latest/linting_specific_commits/) — `--commit`/`--commits` によるコミット範囲指定方法を確認。
- [gitlint 公式ドキュメント: Built-in Rules](https://jorisroovers.com/gitlint/latest/rules/builtin_rules/) — T1〜T8, B1〜B5等の組み込みルール名・ID・既定値・「全ルールが既定で有効」という記載を確認。
- [gitlint 公式ドキュメント: Contrib Rules](https://jorisroovers.com/gitlint/latest/rules/contrib_rules/) — `contrib-title-conventional-commits`(CT1) を含む4つの contrib ルール、有効化方法を確認。
- [gitlint 公式ドキュメント: User Defined Rules](https://jorisroovers.com/gitlint/latest/rules/user_defined_rules/) — `extra-path` によるユーザー定義 Python ルールのロード方法を確認。
- [gitlint GitHub: `gitlint-core/gitlint/rules.py`](https://github.com/jorisroovers/gitlint/blob/main/gitlint-core/gitlint/rules.py)（SHA: f49cb1c2f9da30e6991ce5c86f0f2927662afbfe） — `Rule`/`CommitRule`/`LineRule`/`ConfigurationRule` の継承構造、全組み込みルールクラスの実装、`RuleViolation` にseverityフィールドが無いことを確認。
- [gitlint GitHub: `gitlint-core/pyproject.toml`](https://github.com/jorisroovers/gitlint/blob/main/gitlint-core/pyproject.toml) — `dynamic = ["version", "urls"]`（hatch-vcs によるバージョン管理）を確認。
- GitHub API: `repos/jorisroovers/gitlint/tags` — 最新タグ `v0.19.1` を確認。

## 確認できなかった事項（ギャップ）

- commitlint・gitlint いずれの公式サイトにも、閲覧した範囲内ではモノレポ対応を正面から扱う専用ページが見つからなかった。commitlint 側は過去に該当ページが存在した可能性があるが、現行サイトマップには含まれていない。
- gitlint について、公式ドキュメント・README の範囲で確認した限り、commitlint の `@commitlint/prompt-cli` や Commitizen アダプタに相当する対話的なコミット作成支援の機能は見当たらなかった。「存在しない」と断定するところまでは一次資料から直接証明できないため、「見当たらなかった」という表現にとどめた。
- gitlint の正確なリリースバージョン文字列は `pyproject.toml` が `dynamic` 指定のため、リポジトリのソースから固定のバージョン番号を直接読み取ることはできず、GitHub のタグ名（`v0.19.1`）を代用根拠とした。
- pipx 等の代替インストール手段について、gitlint 公式トップページで確認できたのは `pip install gitlint` のみで、pipx 向けの公式記載は本調査で閲覧した範囲には無かった。
- npm 上の shareable config やプラグインの具体的な一覧については、commitlint 公式サイトには命名規則とメカニズムの説明はあるが、公式ドキュメント内に主要な shareable config / プラグインを一覧化したインデックスページ（`reference/community-projects.md` 等）があることまでは把握したが、今回は時間の都合でその一覧ページの内容までは深掘りしていない。

[^cl-getstart]: https://commitlint.js.org/guides/getting-started.html
[^cl-config]: https://commitlint.js.org/reference/configuration.html
[^cl-conv-index]: https://github.com/conventional-changelog/commitlint/blob/master/%40commitlint/config-conventional/src/index.ts
[^cl-rules]: https://commitlint.js.org/reference/rules.html
[^cl-plugins]: https://commitlint.js.org/reference/plugins.html
[^cl-local]: https://commitlint.js.org/guides/local-setup.html
[^cl-ci]: https://commitlint.js.org/guides/ci-setup.html
[^cl-cli]: https://commitlint.js.org/reference/cli.html
[^cl-share]: https://commitlint.js.org/concepts/shareable-config.html
[^cl-prompt]: https://commitlint.js.org/guides/use-prompt.html
[^cl-cli-pkg]: https://github.com/conventional-changelog/commitlint (GitHub API: `@commitlint/cli/package.json`, `"version": "21.2.4"`)
[^cl-releases]: https://github.com/conventional-changelog/commitlint/releases
[^cl-404]: https://commitlint.js.org/guides/monorepo.html , https://commitlint.js.org/concepts/monorepo.html (いずれも404確認)
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

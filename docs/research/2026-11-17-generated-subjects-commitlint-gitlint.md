# Git 生成件名に対する commitlint・gitlint の扱い

- 調査日: 2026-11-17。対象は [issue #7](https://github.com/sgryjp/commit-msg-lint/issues/7)。
- 対象バージョン: commitlint `v21.2.4`、`@commitlint/cli` と
  `@commitlint/config-conventional` も 21.2.4。gitlint `v0.19.1`。調査時の GitHub 最新リリースを
  固定し、公式ドキュメントと当該タグのソースを照合した[^cl-release][^gl-release]。可変の公式サイトは
  説明の参照用であり、実装上の判定は固定タグを優先する。
- 実測は一時ディレクトリ内の Node.js 26.13.1、Python 3.9.6、Git 2.54.0 で行った。各ツールと
  設定は隔離して導入し、プロジェクトやグローバル設定は変更していない。以下の実測は網羅試験ではない。

## 要約

- 両者とも既定では、一定の特殊メッセージを **対象件名として検査するのではなく、通常の lint 全体から
  除外**する。`amend!` と入れ子も対象になり得る。特に `amend!` の本文は autosquash 後のメッセージに
  置き換わり得るため、「一時コミットだから全内容を捨てる」とは限らない[^git-commit][^cl-lint][^gl-lint]。
- 除外判定は生成元の証明ではない。例えば `amend!NOT CONVENTIONAL` や対象がない `fixup!` も両者の
  既定判定に一致する。gitlint の履歴入力の merge 判定だけは親の数に基づき、メッセージ入力の場合と
  異なる[^cl-defaults][^gl-git]。
- 除外を無効にしても、両者は自動的に接頭辞を除いて対象を lint しない。実測では
  `amend! feat: good` は conventional 設定の commitlint で `type-empty` / `subject-empty`、
  CT1 を有効にした gitlint で CT1 違反となった。個別ルールの例外は別に存在する[^cl-lint][^gl-lint][^gl-rules]。
- 除外をそのまま採用すると操作が容易になる一方、無関係な件名、本文、長さの警告・エラーも見逃す。
  本プロジェクトの表示幅・警告とエラーの区別という設計方針とは緊張する。部分的な緩和や対象の検査は
  見逃しを減らせるが、どの部分をどのルールで検査するか、入れ子や `amend!` 本文をどう扱うかの仕様が
  必要である。これは選択肢の整理であり、issue #7 の結論ではない[^project-design]。

## 1. Git の生成形と入れ子

`git commit --fixup=<commit>` は `fixup! <対象の件名>`、`--fixup=amend:<commit>` と
`--fixup=reword:<commit>` は `amend! <対象の件名>`、`--squash=<commit>` は
`squash! <対象の件名>` を作る。通常の fixup は対象のログメッセージを保持するが、amend/reword は
編集したログメッセージで対象のものを置き換える。squash には追加メッセージも指定できる[^git-commit]。
Git の autosquash はスペースを含む `fixup! `、`amend! `、`squash! ` を認識する[^git-rebase][^git-sequencer]。

対象自体が特殊件名の場合を Git 2.54.0 で実測した。元の `feat: original` に対する fixup を対象に、
さらに `--fixup`、`--fixup=amend:`、`--squash` を作ると、それぞれ
`fixup! fixup! feat: original`、`amend! fixup! feat: original`、
`squash! fixup! feat: original` になった。`rebase -i --autosquash --root` の todo には元の
`pick` の後にそれぞれ `fixup`、`fixup -C`、`squash` が並んだ。これはこの具体例の実測であり、
任意の入れ子や曖昧な対象の解決を一般化するものではない[^git-rebase]。

Git 2.54.0 の sequencer は通常 `Revert "<元の件名>"` と本文を作り、revert の revert では
`Reapply "..."` にする[^git-revert-src]。merge メッセージにも複数の形があるため、単一の
接頭辞ですべての実際の merge を証明することはできない[^git-merge]。

## 2. commitlint: コアの ignore と conventional 設定

### 判定と作用

`@commitlint/lint` はパース・ルール実行より先に、入力末尾だけを `trimEnd()` した文字列を
`@commitlint/is-ignored` に渡す。一致すると `valid: true`、errors / warnings とも空で返し、
対象件名も本文も、設定・プラグインのルールも検査しない。先頭は trim しない。ignore は
`config-conventional` のルールではなくコアの既定動作である[^cl-lint][^cl-ignored]。

主な既定 matcher は次のとおり。表は設定なしの matcher の挙動であり、Git が実際に生成したと
認証するものではない[^cl-defaults]。

- **fixup / amend / squash**: `/^(amend\|fixup\|squash)!/`。大文字小文字を区別する。
  空白・対象・行末は要求せず、入れ子の場合も外側の接頭辞だけを照合する。
- **revert / reapply**: `/^(R\|r)evert (.*)/` と `/^(R\|r)eapply (.*)/`。
  先頭文字だけ大小を許し、後続に ASCII スペースが必要。本文や hash は要求しない。
  `revert: ...` はこの matcher に一致しない。
- **merge**: `Merge pull request`、`Merge ... into ...`、`Merge branch ...`、`Merge tag ...` のほか、
  `Merged ...`、`Automatic merge...`、`Auto-merged...` などに対する matcher がある。正確な式はリンク先を参照。
- **その他**: 先頭行の semver と `chore:` / `chore(scope):`、CI skip 表記を組み合わせた形なども除外する。
  特殊 Git 件名だけの仕組みではない。

merge の最初の二つは `m` フラグ付きであり、文字列先頭ではなく**本文中の行にも一致し得る**。
実測で `bad title\n\nMerge branch evil\n` は先頭が不正でも終了コード 0 だった。前置空白、
改行、コメントの位置で判定が変わり得るため、全 matcher を単純な「件名先頭一致」とみなせない[^cl-defaults]。

公式設定の `defaultIgnores: false` は既定 matcher **全体**を外す。`ignores: [(message) => ...]` は
任意の predicate を追加し、既定と OR で評価される。既定を無効にして必要なものだけ自作することも
できる。カスタム ignore は `defaultIgnores: false` でも残る[^cl-config][^cl-ignored]。

### 入力・ルールセット

CLI は通常 stdin、`--edit <file>` ではファイル、`--last` / `--from` / `--to` などでは履歴を読む。
フック例は `commitlint --edit "$1"` である。いずれもメッセージごとに同じ ignore 判定を通り、
実際の Git 操作や親数で特殊扱いするわけではない。`--edit` 時はパーサーに `core.commentChar` を
設定するが、ignore はそのパースより前なので、同一のフィルタ済み入力と決めつけない[^cl-cli][^cl-local][^cl-lint]。

CLI にルールがない場合、ignore に一致しても CLI は `empty-rules` エラーを返す。このバージョンの
`--default-config` は設定解決後にルールがない場合の conventional 設定へのフォールバックであり、
常に暗黙適用されるのではない。通常は `extends: ['@commitlint/config-conventional']` などを
明示する[^cl-cli][^cl-config]。

conventional 設定は `header-max-length` を error / 100、`body-leading-blank` と
`footer-leading-blank` を warning とし、type・subject の非空なども設定する。ignore はそれらの前で
返るため、長さも warning も出ない。ignore を切ると元のメッセージに設定ルールを適用する。
実測では `amend! feat: good` は `type-empty` / `subject-empty` で失敗し、
`Revert "feat: good"` に `This reverts commit abc123.` を付けた場合も同じ二つで失敗した。
一方、`revert: feat: good` は同設定で成功した。これはこの設定と入力での結果であり、
別パーサー・別ルールの一般的な結果ではない[^cl-conventional][^cl-lint]。

## 3. gitlint: コア判定、入力コンテキスト、ルール

### 判定と作用

既定の `ignore-merge-commits`、`ignore-revert-commits`、`ignore-fixup-commits`、
`ignore-fixup-amend-commits`、`ignore-squash-commits` はすべて `true`。lint はまず configuration
rules を適用し、その後これらのいずれかに一致すると空の違反リストを返す。従って title / body / commit
の各検査ルールは実行されず、対象も本文も検査されない。configuration rules は先に実行される点で
commitlint と処理順が異なる[^gl-config-src][^gl-lint]。

stdin、`--msg-filename`、`--staged` によるメッセージ入力では、コメント行・シザー行を処理した
後の title に対して、次の大文字小文字を区別する `startswith` を使う[^gl-git]。

| 種別 | title の先頭 |
| --- | --- |
| fixup / amend / squash | `fixup!` / `amend!` / `squash!`。空白・対象は不要。外側だけを判定。 |
| merge | `Merge`。空白不要なので `Merged nonsense` も一致。 |
| revert | `Revert`。空白不要なので `Revert: foobar` も一致。`revert ...` や `Reapply ...` は一致しない。 |

重要な例外として、履歴から得た `LocalGitCommit` の merge 判定は title でなく **親が 2 個以上**かで
決まる。他の特殊種別は title 判定のままである。したがって同じ `Merge arbitrary` というメッセージも、
stdin では除外されるが、親が一つの履歴コミットでは merge として除外されない。逆に、非典型的な件名の
実際の merge は履歴入力では除外され得る。`--staged` は履歴コミットではなく title 判定を使う[^gl-git][^gl-cli]。

`--msg-filename` が stdin に優先し、空でない stdin が次、それ以外は履歴の HEAD / `--commit` /
`--commits` にフォールバックする。gitlint のインストールする `commit-msg` フックと pre-commit 定義は
`--staged --msg-filename` を使う[^gl-cli][^gl-hook][^gl-precommit]。

### 設定とルールセット

5 種の `[general]` オプションは個別に `false` にできる。たとえば
`gitlint -c general.ignore-fixup-amend-commits=false` とする。`ignore-by-title` は正規表現に
合う title の全ルールまたは指定ルールを除外でき、メッセージ内の `gitlint-ignore` など他の除外機構も
ある。全体除外を解除しても他の除外や個別ルールの設定が残り得る[^gl-options][^gl-ignore]。

既定の組み込み規則は一般的な書式であり、Conventional Commits の CT1 は opt-in contrib である。
例えば既定 T1 は title 最大 72 文字。全体除外中は T1 も CT1 も通らない。全体除外を解除した場合も、
組み込み B6 `body-is-missing` の `ignore-merge-commits` は別オプションで、既定 `true` のままなら
merge に B6 を適用しない[^gl-builtin][^gl-contrib][^gl-rules]。

contrib CC2 `contrib-disallow-cleanup-commits` は fixup / amend / squash を拒否するルールだが、
有効にするだけでは全体除外が先に返る。該当する 3 種の general ignore も `false` にする必要がある。
CC2 の実装は title 中の任意位置ではなく前述の `is_*` 判定を使う[^gl-cc2][^gl-lint]。
実測では、全体除外を切り CT1 を有効にすると `fixup! feat: good` は CT1 違反、本文なしの場合は
B6 違反にもなった。`squash! fixup! invalid` も対象を剥がさず CT1 に失敗した。

## 4. 境界例の実測とバージョン上の注意

次は stdin から同一の一行メッセージを渡した結果である。commitlint は `--default-config`、gitlint は
既定設定を使用した。「除外」は終了コード 0 に加え、各ソースの分岐に一致することを確認したもの。
「非除外」はこの入力・設定で違反が出たことを指し、すべての設定での失敗を意味しない[^cl-defaults][^gl-git][^gl-lint]。

| 入力 | commitlint | gitlint |
| --- | --- | --- |
| `fixup!` | 除外 | 除外 |
| `amend!NOT CONVENTIONAL` | 除外 | 除外 |
| `squash! fixup! invalid` | 除外 | 除外 |
| `Fixup! feat: valid` | 非除外 | 非除外 |
| `Merge arbitrary` / `Merged nonsense` | 非除外 | 除外 |
| `Merge branch demo` | 除外 | 除外 |
| `Revert: foobar` | 非除外 | 除外 |
| `revert "feat: x"` / `Reapply "feat: example"` | 除外 | 非除外 |

commitlint の `amend!` ignore は `@commitlint/is-ignored` 19.2.2、`Reapply` ignore は 19.6.0 で
追加された。gitlint の公式履歴では merge 0.7.0、fixup / squash 0.9.0、revert 0.13.0、
fixup=amend 0.18.0 からである。古い導入先に現在の挙動を適用してはならない[^cl-changelog][^gl-history]。

実際にどの Git 操作が `commit-msg` を呼ぶかは、ツールの matcher とは別問題である。Git はフックを
スキップでき、すべての履歴生成経路が同じフックを通るとは限らない。特に sequencer の経路は
[既存の Git 入力調査](2026-11-13-git-commit-message-input.md)を参照する。今回、全 Git バージョン・ <!-- rumdl-disable-line MD089 -->
操作のフック起動や、全パーサー・ルール設定での挙動は再実測していない[^git-hooks][^git-commit]。

## 出典

[^cl-release]: <https://github.com/conventional-changelog/commitlint/releases/tag/v21.2.4>
[^gl-release]: <https://github.com/jorisroovers/gitlint/releases/tag/v0.19.1>
[^git-commit]: <https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---fixupamendrewordltcommitgt>
[^git-rebase]: <https://git-scm.com/docs/git-rebase#Documentation/git-rebase.txt---autosquash>
[^git-sequencer]: <https://github.com/git/git/blob/v2.54.0/sequencer.c#L6646-L6648>
[^git-revert-src]: <https://github.com/git/git/blob/v2.54.0/sequencer.c#L5620-L5635>
[^git-merge]: <https://git-scm.com/docs/git-fmt-merge-msg>
[^git-hooks]: <https://git-scm.com/docs/githooks#_commit_msg>
[^project-design]: [Design principles](../design-principles.md)
[^cl-defaults]: <https://github.com/conventional-changelog/commitlint/blob/v21.2.4/%40commitlint/is-ignored/src/defaults.ts>
[^cl-ignored]: <https://github.com/conventional-changelog/commitlint/blob/v21.2.4/%40commitlint/is-ignored/src/is-ignored.ts>
[^cl-lint]: <https://github.com/conventional-changelog/commitlint/blob/v21.2.4/%40commitlint/lint/src/lint.ts>
[^cl-config]: <https://commitlint.js.org/reference/configuration.html>
[^cl-cli]: <https://github.com/conventional-changelog/commitlint/blob/v21.2.4/%40commitlint/cli/src/cli.ts>
[^cl-local]: <https://commitlint.js.org/guides/local-setup.html>
[^cl-conventional]: <https://github.com/conventional-changelog/commitlint/blob/v21.2.4/%40commitlint/config-conventional/src/index.ts>
[^cl-changelog]: <https://github.com/conventional-changelog/commitlint/blob/v21.2.4/%40commitlint/is-ignored/CHANGELOG.md>
[^gl-git]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/gitlint-core/gitlint/git.py>
[^gl-lint]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/gitlint-core/gitlint/lint.py>
[^gl-config-src]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/gitlint-core/gitlint/config.py>
[^gl-cli]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/gitlint-core/gitlint/cli.py>
[^gl-hook]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/gitlint-core/gitlint/files/commit-msg>
[^gl-precommit]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/.pre-commit-hooks.yaml>
[^gl-options]: <https://jorisroovers.com/gitlint/0.19.x/configuration/general_options/>
[^gl-ignore]: <https://jorisroovers.com/gitlint/0.19.x/ignoring_commits/>
[^gl-builtin]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/docs/rules.md>
[^gl-rules]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/gitlint-core/gitlint/rules.py>
[^gl-contrib]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/docs/contrib_rules.md>
[^gl-cc2]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/gitlint-core/gitlint/contrib/rules/disallow_cleanup_commits.py>
[^gl-history]: <https://github.com/jorisroovers/gitlint/blob/v0.19.1/docs/index.md#merge-fixup-squash-and-revert-commits>

# gitlint パーサー実装設計 調査報告

- 調査日: 2026-11-09
- 対象バージョン: `gitlint-core` main ブランチ(最終コミット `4d91197`, 2023-09-02)。
  PyPI 上の最新リリースは 0.19.1(2023-03-10)で、パーサー部分に差はない。
- 方法: `jorisroovers/gitlint` のソースを読み、Python 3.13 / Git 2.43 上
  で `GitCommitMessage.from_full_message` と `GitLinter.lint` を直接呼んで挙動を確認した。
- 備考: main は 2023-09 以降、依存更新以外のコミットがなく、リリースも 0.19.1 で止まっている。

## 1. 要約

- gitlint のパーサーは `gitlint/git.py` の `GitCommitMessage.from_full_message` という
  **約 15 行の静的メソッド** だけである。文法、トークナイザー、状態機械のいずれもない。
- 処理は「`str.splitlines()` で行分割 → シザー行以降を切り捨て → コメント文字で始まる行を除去 →
  1 行目を title、残りを body」で終わる。**出力は `title: str` と `body: list[str]` の 2 要素** で、
  footer やトレーラーの概念は存在しない。
- Conventional Commits はパーサーではなく、contrib ルール
  `CT1`(`contrib-title-conventional-commits`)が title 1 行に正規表現を当てて検査するだけである。
  type / scope / subject は構造として取り出されない。
- 位置情報は「行番号」のみで、それもパーサーではなく lint エンジンがルール呼び出し時に
  `1`(title) と `2` 以降(body)を振る。コメント行除去後の行番号なので、
  **元ファイル上の行番号とずれる**。列情報はない。
- 長さは Python の `len()`(Unicode コードポイント数)で測る。全角文字も絵文字も 1 である。

## 2. パッケージ構成と呼び出し経路

```
gitlint CLI (cli.py, click)
  └ build_git_context()        入力源を決める: --msg-filename > stdin > ローカルリポジトリ (git log)
       └ GitContext.from_commit_msg / from_staged_commit / from_local_repository
            └ GitCommitMessage.from_full_message(context, text)   ← パーサー本体
  └ GitLinter.lint(commit)     (lint.py)
       ├ ConfigurationRule.apply()      I1〜I4: 設定やメッセージ自体を書き換える
       ├ 特殊コミット判定でスキップ       merge / squash / fixup / fixup_amend / revert
       ├ LineRule (target=Title)         title 1 行に適用、line_nr=1
       ├ LineRule (target=Body)          body の各行に適用、line_nr=2,3,...
       └ CommitRule                       コミット全体に適用、line_nr はルールが任意に決める
```

| ファイル | 行数 | 役割 |
|---|---|---|
| `git.py` | 466 | git コマンド呼び出し、`GitContext`、`GitCommitMessage`(パーサー)、`GitCommit` 系 |
| `lint.py` | 128 | ルール分類と適用、行番号付与、違反のソート |
| `rules.py` | 493 | 組み込みルール(T1〜T8, B1〜B8, M1, I1〜I4) |
| `contrib/rules/*.py` | — | CT1(Conventional Commits)、CC1(Signed-off-by)、CC2、CC3 |

パーサーと git コマンド実行が同じモジュールにあり、`GitCommitMessage` はコメント文字を得るために
`GitContext` を引数に取る。
`GitContext.commentchar` は `git config --get core.commentchar` を実行して得るため、
**パースに git プロセスの起動が絡む**(未設定時は `#`)。

## 3. `GitCommitMessage` の設計

### 3.1 パース処理(全文)

```python
all_lines = commit_msg_str.splitlines()
cutline = f"{context.commentchar} ------------------------ >8 ------------------------"
try:
    cutline_index = all_lines.index(cutline)
except ValueError:
    cutline_index = None
lines = [line for line in all_lines[:cutline_index] if not line.startswith(context.commentchar)]
full = "\n".join(lines)
title = lines[0] if lines else ""
body = lines[1:] if len(lines) > 1 else []
```

### 3.2 出力モデル

```python
@dataclass
class GitCommitMessage:
    context: GitContext
    original: str       # 入力そのまま
    full: str           # コメント除去後を "\n" で再結合したもの
    title: str          # 除去後の 1 行目
    body: list[str]     # 除去後の 2 行目以降(ヘッダー直後の空行も要素 "" として含む)
```

特徴:

- **title と body の 2 分割しかない**。空行による段落区切り、footer、トレーラーは
  いずれも表現されない。`BREAKING CHANGE:` や `Signed-off-by:` も body の 1 行にすぎない。
- body の先頭要素は通常、ヘッダー直後の空行 `""` である。空行は区切りとして消費されず、
  データとして残る。そのため body を扱うルールは「`body[0]` が空かどうか」「`body[1:]` から本文」
  といった約束を各自で持っている(B4, B6, B8)。
- `original` は保持されるが、`title`/`body` の各要素が `original` のどこに対応するかの対応表はない。

### 3.3 前処理の詳細

1. **行分割は `str.splitlines()`**。`\n` / `\r\n` / `\r` に加え、`\v` `\f` `\x1c`〜`\x1e` `\x85`
   `U+2028` `U+2029` でも分割される。Git 自身は `\n` だけを行区切りとするため、
   これらを含むメッセージでは Git と行の認識が食い違う(実測 4 章)。
2. **シザー行**は `"{commentchar} ------------------------ >8 ------------------------"` との
   **行全体の完全一致** で探す。インデントされたシザー行は無視する。
   Git の `commit --verbose` / `cleanup=scissors` と同じ文字列である。
3. **コメント行**は `line.startswith(commentchar)` で除去する。commit-msg フックは
   クリーンアップ前のファイルを受け取るため、この処理で Git のテンプレートコメントを除く。
   `--commits` で既存コミットを読む場合も同じ処理が走るため、
   `cleanup=verbatim` で `#` 始まりの行を持つコミットでは本文が欠ける。
4. 先頭・末尾の空行は除去しない。先頭に空行があると title が `""` になる。
   末尾の改行は `splitlines()` の性質上、空要素を生まない。
5. 入力の妥当性検査はなく、空文字列でも例外にならない(title=`""`, body=`[]`)。

### 3.4 コメント文字の扱い

`core.commentChar` の値を文字列としてそのまま前置一致に使う。そのため:

- `;` などの単一文字は正しく動く。
- Git 2.45 以降の複数文字のコメント文字列も、前置一致なので結果的に動く。
- **`core.commentChar=auto` は未対応**。Git は `auto` のとき本文で未使用の文字を選ぶが、
  gitlint は文字列 `"auto"` で始まる行をコメントとして除去する(実測 4 章)。

## 4. 実測で確認した挙動

既定設定 + `contrib=contrib-title-conventional-commits` で確認した
(コメント文字は明記しない限り `#`)。

| 入力 | 結果 | 所見 |
|---|---|---|
| `feat(api)!: drop v1` + 本文 | 違反なし | `!` は CT1 の正規表現で許容されるだけで、破壊的変更として扱われない |
| `Update readme` | `1: CT1` 形式不一致 | ヘッダー不一致はパースエラーではなくルール違反 |
| `feat(a(b)): x ...` | `1: CT1` 形式不一致 | scope は `[^)]+?` で入れ子の括弧を許さない(commitlint は `a(b)` を scope とする) |
| `feat:no space` / `feat(): x` | `1: CT1` 形式不一致 | `: ` の空白必須、空 scope 不可 |
| `Feat: x` / `feat (api): x` | `1: CT1` 許可 type でない | type 部は `[^(]+?` で空白も含みうる。`feat (api)` の type は `"feat "` と解釈され、「type が不正」という的外れなメッセージになる |
| `fix: c\nbody immediately` | `2: B4` 2 行目が空でない、`3: B6` 本文なし | 空行欠落を B4 が検出。body が 1 要素なので B6 も同時に出る |
| `\nfix: leading blank` | title=`""`、`1: CT1`、`1: T8`、`2: B4` | 先頭空行を除去しないため、実際のヘッダーが body に落ちる |
| `#123 fix the issue` + 本文 | title=`""` | issue 番号で始まる件名がコメントとして消える(Git のクリーンアップと同じ結果ではある) |
| `fix: x\n# a comment\n\nbody(95 文字)` | `3: B1` | 長すぎる行は元ファイルの 4 行目だが、コメント除去後の番号 3 で報告される |
| シザー行 + 差分 | シザー以降は無視 | 完全一致時のみ。インデント付きシザー行は本文として残る |
| CRLF 改行 | 正常 | `splitlines()` が `\r\n` を吸収する |
| 件名に `U+2028` を含む | title が分割され、後半が body の 1 行目に | Git は 1 行とみなす |
| `feat(ui): ` + 全角 45 文字 | 違反なし(len=55) | EAW 幅では 100 桁だが、コードポイント数で測るため T1(72)に掛からない |
| 件名に絵文字 2 つ | 各 1 文字として計数 | サロゲートペアの問題はない(UTF-16 長の commitlint とは異なる) |
| `core.commentChar=auto`、本文に `auto this line ...` | その行が除去される | `auto` を文字列として前置一致させている |
| `core.commentChar=;` | `;` 行が除去、`#` 行は残る | 正しく追従する |
| 空文字列 | `1: CT1`、`1: T8`、`3: B6` | 存在しない 3 行目を指す違反が出る |
| `BREAKING CHANGE: ...` と `Signed-off-by: ...` を含む | 本文の 2 行として扱われる | footer の概念がない |
| `ignore-body-lines` で `Signed-off-by` 行を除外 | 後続の長い行が `4: B1`(元は 5 行目) | 設定ルールが body を書き換え、行番号がさらにずれる |

## 5. ルール層の設計

### 5.1 ルールの種類と適用単位

| 基底クラス | 入力 | 行番号 | 例 |
|---|---|---|---|
| `LineRule` + `target=CommitMessageTitle` | title 文字列 | エンジンが `1` を付与 | T1〜T8, CT1 |
| `LineRule` + `target=CommitMessageBody` | body の各行 | エンジンが `2, 3, ...` を付与 | B1〜B3 |
| `CommitRule` | `GitCommit` 全体 | ルールが自分で決める(なしも可) | B4〜B8, M1, CC1 |
| `ConfigurationRule` | 設定と `GitCommit` | なし | I1〜I4 |

`LineRuleTarget` は title と body の 2 種類だけで、「footer の各行」「type 部分」のようなターゲット
は定義できない。ユーザー定義ルール(`--extra-path` で読む Python クラス)も同じ枠組みに従う。

### 5.2 行番号の決まり方

- `LineRule` の行番号は `_apply_line_rules` が `title=1`、`body[i]=i+2` として振る。
  元テキストの行番号ではなく **コメント除去後の配列添字** である。
- `CommitRule` はルールごとにまちまちである。B5・B6 は常に `3`、B7・B8 は
  `len(body)+1`(最終行の次)、CC1 は `1`、M1 は `None`。意味の統一はない。
- 違反の位置は行番号のみで、列・範囲はない。`RuleViolation.content` に違反行の文字列全体を入れて
  表示しているだけである。

### 5.3 パース結果の書き換え

`IgnoreBodyLines`(I3)は `commit.message.body` と `full` を直接書き換える。パース結果がルール実行中に
変更される設計であり、以後のルールが見る body は元の body ではない。行番号のずれはここでも生じる。

### 5.4 特殊コミットの判定

`is_merge_commit` などは title の前置一致 (`"Merge"`, `"fixup!"`, `"squash!"`, `"amend!"`,
`"Revert"`)で判定する。
ローカルリポジトリから読む `LocalGitCommit` だけは親コミット数でマージを判定し直す。
判定は lint エンジンで行われ、該当すれば全ルールを飛ばす。

### 5.5 長さ計測

`MaxLineLength.validate` は `len(line) > max_length` だけである。title 既定 72、body 既定 80。
URL 免除のような例外はなく、必要なら `ignore-body-lines` で行ごと除外する。

## 6. 設計上の評価

### 利点

- **極端に単純**: パーサーは 15 行で、挙動を誤解する余地がほとんどない。
- **Git との整合を意識した前処理**: `core.commentChar` を git から読み、
  シザー行も Git の文字列に合わせている(`auto` を除く)。
- **寛容**: どんな入力でも title/body を返し、判断はすべてルールに委ねる。
- **長さ計測がコードポイント単位**: UTF-16 長の commitlint より直感に近い(ただし表示幅ではない)。
- **ルール拡張の敷居が低い**: 行単位・コミット単位のルールを Python クラスで足せる。

### 欠点

- **構造が浅すぎる**: footer、トレーラー、段落、ヘッダー内部の構造がない。Conventional Commits
  を検査するには、各ルールが title や body を独自の正規表現で再解析する必要があり、
  CT1 と CC1 は実際にそうしている。
- **位置情報が不正確**: コメント行の除去と I3 による書き換えで、報告される行番号が
  元ファイルとずれる。CommitRule の行番号は慣習的な固定値で、存在しない行を指すこともある。
  エディタ連携には使えない。
- **行分割が Git と異なる**: `splitlines()` は Unicode の行区切りでも分割する。
- **パースが git プロセスに依存する**: コメント文字の取得で `git config` を呼ぶため、純粋関数として
  テストしにくい(テストではモックしている)。
- **パース結果が可変**: 設定ルールが body を書き換えるため、ルールの実行順に結果が依存する。
- **Conventional Commits 検査が弱い**: CT1 の正規表現は type に任意の文字(空白含む)を許し、
  scope の入れ子を許さず、`!` や footer の `BREAKING CHANGE` を区別しない。
- **保守状況**: 2023 年以降、機能開発が止まっている。

## 7. commitlint との比較

| 観点 | commitlint (`conventional-commits-parser`) | gitlint |
|---|---|---|
| パーサーの規模 | 状態機械 466 行 + 正規表現生成 | 15 行 |
| 出力 | header / type / scope / subject / body / footer / notes / references ほか | title / body(行配列) |
| Conventional Commits の知識 | パーサープリセット(正規表現)で注入 | contrib ルール CT1 の正規表現のみ |
| footer 判定 | footer トークン行で一方向遷移(本文中の `Note:` で誤遷移) | footer 概念なし |
| 空行の扱い | 区切りとして使わない。`body-leading-blank` が `raw` を再分割 | body の要素 `""` として残し、B4 が `body[0]` を見る |
| 位置情報 | なし | 行番号のみ(除去後の番号でずれる) |
| 長さの単位 | UTF-16 コードユニット | Unicode コードポイント |
| コメント文字 | `commentChar` オプション(設定で指定) | `git config core.commentchar` を実行して取得 |
| 結果の可変性 | 不変(`raw` を付けて返すだけ) | I3 が body を書き換える |

両者とも、パース結果の不足をルール側の再解析で補っている点は共通である。
commitlint は「構造は豊富だが位置がない」、gitlint は「構造も位置もほぼない」と言える。

## 8. Rust での再実装に向けた示唆

commit-msg-lint(Rust、EAW 行長、CLI/LSP 両対応)の観点で、
commitlint 調査の結論に追加・補強する点を整理する。

**引き継ぐ価値がある点**

- **コメント文字を Git の設定から得る** 方針。ただし `auto` は Git と同じ規則
  (本文に現れない候補文字を選ぶ)で解決し、複数文字のコメント文字列(`core.commentString`)も扱う。
  git プロセスの起動はパーサーの外で行い、パーサーにはコメント文字列を値として渡す。
- **シザー行の完全一致判定**。インデント付きは本文扱いという Git の挙動に合わせる。
- **ヘッダー直後の空行欠落をパース失敗にしない**。gitlint の B4 のように、
  ルールが「2 行目が空でない」を診断する形がよい。
- **title(ヘッダー行)単位のルールと body 行単位のルールを分ける** 適用モデル。
  行単位ルールにエンジンが位置を付与する仕組みは、行番号ではなくバイト範囲に拡張すれば
  LSP にそのまま使える。

**避けるべき点**

- **行を除去してから番号を振る**: gitlint の行番号ずれはこれが原因である。コメント行・シザー以降は
  トリビアとして木に残し、各ノードが元テキスト上の範囲を持つようにする。
- **`str.splitlines()` 相当の行分割**: Rust の `str::lines()` は `\n` と `\r\n` のみで分割するので
  Git と整合する。`U+2028` などは行内文字として扱い、必要なら別のルールで警告する。
- **先頭空行をそのままヘッダー扱いにする**: Git はクリーンアップで先頭空行を除くため、commit-msg
  フック入力では先頭空行をスキップしてヘッダーを探す(スキップした空行はトリビアとして保持する)。
- **パース結果をルールが書き換える設計**: 行の除外(gitlint の I3 相当)は、パース結果を変えずに
  「このルールの対象から外す」フィルタとして実装する。
- **CommitRule が行番号を慣習値で返す設計**: コミット全体の違反でも、診断を出す範囲(ヘッダー、
  body 全体、トレーラーブロック、ファイル先頭など)を明示的に選ばせる。
- **Conventional Commits を title の正規表現 1 本で検査する方式**: type に空白を含めてしまう、
  scope の入れ子を区別できない、といった CT1 の問題は、ヘッダーの字句解析で避けられる。
  誤りの種類ごとに範囲付きの診断を出せるようにする。

**長さ計測について**

gitlint はコードポイント数、commitlint は UTF-16 長で、どちらも表示幅ではない。commit-msg-lint の
EAW 幅計測は両者に対する明確な差別化点になる。比較のため、計測関数を「コードポイント数」
「EAW 幅(A を 1 または 2)」で切り替えられるようにしておくと、
既存ツールからの移行時に同じ基準での比較もできる。

## 9. 参照したソース

- `gitlint-core/gitlint/git.py`(`GitContext`, `GitCommitMessage.from_full_message`, `git_commentchar`, `GitCommit` 系)
- `gitlint-core/gitlint/lint.py`(`GitLinter`, `_apply_line_rules`)
- `gitlint-core/gitlint/rules.py`(T1〜T8, B1〜B8, M1, I1〜I4)
- `gitlint-core/gitlint/contrib/rules/conventional_commit.py`(CT1)
- `gitlint-core/gitlint/contrib/rules/signedoff_by.py`(CC1)
- `gitlint-core/gitlint/cli.py`(`build_git_context`, `get_stdin_data`)
- `gitlint-core/gitlint/utils.py`(`FILE_ENCODING`)
- `CHANGELOG.md`

リポジトリ:
- [jorisroovers/gitlint](https://github.com/jorisroovers/gitlint)
- 比較対象: `research/commitlint-parser-design.md`

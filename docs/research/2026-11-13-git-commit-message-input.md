# Git 固有のコミットメッセージ入力 調査報告

- 調査日: 2026-11-13
- 対象バージョン: Git 2.54.0(ソースはタグ `v2.54.0`、ドキュメントは git-scm.com の現行版)。
  バージョンに依存する挙動は、依存するバージョンを明記する。
- 方法: Git 公式ドキュメントと Git 本体のソースを読み、手元の Git 2.54.0(Apple Git-157、macOS)で
  使い捨てリポジトリを作って `commit-msg` フックの入力を実測した。実測した記述には「実測」と書く。
  ソースの読解だけで推論した記述には「ソースからの推論」と書く。
- 目的: commit-msg-lint が将来対応を検討すべき Git 固有の要素の整理。実装方針は決めない。
- 前提となる既存文書: `docs/message-format.md`、`docs/cli.md`、`README.md` の Design philosophy、
  `TODO.md` の「Message compatibility」。過去の調査
  (`2026-11-09-gitlint-parser-design.md`、`2026-11-09-commitlint-parser-design.md`)の
  scissors / `auto` に関する記述は 8 章で検証する。

## 1. 要約

- `commit-msg` フックが受け取るのは「最終コミットメッセージ」ではない。
  `git commit` が `COMMIT_EDITMSG` に書いた **cleanup 前のファイル** であり、
  Git はフックの終了後に cleanup を適用する。
  ファイルの中身は `-m` / `-F` か、エディターを使うかによって大きく異なる。
- cleanup の方式は `--cleanup`、`commit.cleanup`、エディター使用の有無で決まる。
  さらに `-v` / `commit.verbose` が有効だと、方式に関係なく scissors 行以降が切り捨てられる。
  フックの引数はファイルのパスだけで、エディターを開かないときは環境変数 `GIT_EDITOR=:` も渡される。
  **cleanup の方式も `-v` の有無も、フックからは確実に判別できない。**
- コメント行の除去は「エディターを使うときの既定(`strip`)」の挙動にすぎない。
  `-m` / `-F` では既定が `whitespace` になり、`#` 行はそのままコミットに残る(実測)。
  コメント行を無条件に除去する linter は、実際に残るメッセージを誤って検査する。
- コメント文字は `core.commentChar` / `core.commentString` で決まる。
  複数文字は Git 2.45.0 以降で使える。`auto` は非推奨で Git 3.0 で削除される。
  `auto` で Git が選んだ文字は、設定値からもフックの引数からも得られない。
- scissors 行は「コメント文字 + 空白 + `------------------------ >8 ------------------------`」が
  行頭にあり、LF で終わる行だけが一致する(ソース確認、実測)。
- 空メッセージの判定、エンコーディング、自動生成メッセージ、trailers にも、
  Git 側の規則が linter の現行仕様と食い違う箇所がある(5〜7 章)。

## 2. 現行の commit-msg-lint の前提(比較対象)

- 入力は標準入力か `--edit <path>` のファイルで、UTF-8 でなければ入力エラー(終了コード 2)になる。
  CRLF を LF に正規化し、末尾の LF を最大 1 つ除く。コメント除去、Git の cleanup、
  Unicode 正規化は行わない。コメント行や scissors 行も入力としてそのまま検査する。
  (`docs/message-format.md` の「Input and lines」、`docs/cli.md` の「Input」)
- フック用途は `commit-msg-lint --edit "$1"` と案内している(`docs/cli.md`)。
- 診断の行番号は CRLF 正規化後の 1 始まりで、`<source>` は `<stdin>` か `--edit` のパスである。
- `TODO.md` には、コメント行・scissors 行がフック入力で有効なコミットを失敗させうるという課題が
  ある。
  README の Design philosophy は、表示幅の測定と、警告・エラーの区別による判断の余地を掲げている。
  Git の cleanup 互換はこの 2 項目の外にあるため、仕様として追加するなら別途の位置づけが要る。
- 入力経路は 4 種類ある。
  1. フック(`--edit` に Git の作業ファイルを渡す)
  2. 通常ファイル(`--edit`)
  3. 標準入力
  4. Web デモ(`web/src/lib.rs` の `lint_message(message, config_toml)`。ブラウザーに貼った文字列で、
     Git の設定も環境変数もない)

## 3. フックが受け取る入力と最終コミットの差

### 3.1 どのコマンドが、どのファイルで、いつ呼ぶか

- `commit-msg` は `git commit` と `git merge` が呼び、引数は「提案されたコミットログメッセージを
  保持するファイル名」の 1 つだけである。`--no-verify` で省略できる。
  フックはファイルを書き換えてよく、メッセージを拒否してもよい。
  出典: <https://git-scm.com/docs/githooks> の `commit-msg` 節
  (「It takes a single parameter, the name of the file that holds the proposed commit log
  message.」「The hook is allowed to edit the message file in place」)。
- `git commit` では `.git/COMMIT_EDITMSG` を渡す。
  `prepare-commit-msg` → エディター起動 → `commit-msg` の順に呼ぶ。
  `commit-msg` を呼んだ後で、ファイルを読み直して cleanup と空メッセージ判定を行う。
  出典: <https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L1116-L1135>(フックとエディター)、
  <https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L1895-L1927>(読み直しと cleanup)。
- `git merge` は `commit-msg` に `MERGE_MSG` のパスを渡す。`COMMIT_EDITMSG` ではない。
  出典: <https://github.com/git/git/blob/v2.54.0/builtin/merge.c#L974-L977>。
  `merge` の cleanup は `cleanup_message(&msg, cleanup_mode, 0)` で、同じ関数である(同 L980 付近)。
- cherry-pick、revert、rebase のようにシーケンサーが作るコミットは、
  エディターを開く場合(`VERIFY_MSG`)にだけ `git commit` をフック有効で呼び、
  それ以外は `git commit -n`(`--no-verify`)で作る。
  エディターを開かないときは `--cleanup=verbatim` を付ける場合もある。
  出典: <https://github.com/git/git/blob/v2.54.0/sequencer.c#L1122-L1181>
  (`if (!(flags & VERIFY_MSG)) strvec_push(&cmd.args, "-n");`、
  `--cleanup=verbatim`、`--cleanup=strip`)。
  ⇒ これらの操作では、編集しない限り `commit-msg` フックは動かない(ソースからの推論)。
- 環境変数 `GIT_EDITOR` は、エディターを開かないとき `:` に設定される。
  出典: <https://git-scm.com/docs/githooks>(「All the git commit hooks are invoked with the
  environment variable GIT_EDITOR=: if the command will not bring up an editor to modify the
  commit message.」)、実装は
  <https://github.com/git/git/blob/v2.54.0/commit.c#L1970-L1990>
  (`strvec_push(&opt.env, "GIT_EDITOR=:")`)。
  実測では、`-m` のみで `GIT_EDITOR=[:]`、`-m ... -e` で外部エディターを使うと元の値のままだった。
  なお利用者が自分で `GIT_EDITOR=:` を設定した場合は、エディターを使う場合と区別できない。

### 3.2 ファイルの中身(`git commit` の場合)

ファイルは `prepare_to_commit` が次の順で作る
(<https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L925-L932>、L942-L990)。

1. メッセージ本体(`-m` / `-F` / テンプレート / 既存コミット / `MERGE_MSG` など)。
   `cleanup_mode` が `verbatim` でなければ、ここで `strbuf_stripspace(&sb, NULL)` を適用する。
   第 2 引数が `NULL` なので、**空白の整理だけを行い、コメント行は除去しない**。
2. `-s` の `Signed-off-by` を追加する。
3. エディターを使い、かつ `--status`(既定)のとき、コメント文字付きの案内文と status を追加する。
   `cleanup=scissors` なら scissors 行、`strip` なら「Lines starting with '#' will be ignored」、
   `whitespace` なら「Lines starting with '#' will be kept」という案内が入る。
4. `--trailer` の値を `amend_file_with_trailers` で追加する
   (<https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L1070-L1073>)。
   `commit-msg` を呼ぶ前なので、フックはトレーラー追加後の姿を見る。

### 3.3 実測した入力(Git 2.54.0、`commit-msg` で `cat -et`)

- `git commit -m "<空行 2、feat: x␠␠␠、空行 2、# note、body>"`(`GIT_EDITOR` は `:`)
  - フックのファイルは `feat: x` / 空行 / `# note` / `body` だった。
    前後の空行と行末空白は除去済みで、`# note` は残る。
- `git commit -F m.txt --cleanup=verbatim`(本文は `feat: y␠␠`、空行、`# keep`。`GIT_EDITOR` は `:`)
  - 行末空白も `# keep` も入力のままだった。
- `GIT_EDITOR=true git commit -m "feat: z" -e`(`GIT_EDITOR` は `true`)
  - `feat: z` の後に `# Please enter the commit message ...`、`# On branch main`、
    `# Changes to be committed:` などが続いた。
- 上に `--cleanup=scissors -v` を追加(`GIT_EDITOR` は `true`)
  - `feat: s` の後に scissors 行、案内 2 行、status、`diff --git ...` 以降の差分が続いた。
- `-e -v`(cleanup は既定の `strip`、`GIT_EDITOR` は `true`)
  - 案内と status の後に scissors 行と差分が続いた。

実測から確認できること:

- `-m` / `-F` で cleanup が `verbatim` でないとき、フックは行末空白が既に除かれたファイルを見る。
  つまり `line-trailing-whitespace` は、`-m` / `-F` 経由では Git の整理後に評価される。
  `--cleanup=verbatim` では行末空白が残る。
- エディターを使うと、ファイルには Git 自身が追記した多数のコメント行が含まれる。
  これは `strip` ではフックの後に除かれるが、`whitespace` では除かれない(3.4 節)。
- `-v` を指定すると、`strip` でも scissors 行と差分が追記される(上の最終行)。

### 3.4 フックの入力と最終メッセージの差

最終メッセージは `commit-msg` の後に `cleanup_message` が作る。
出典: <https://github.com/git/git/blob/v2.54.0/sequencer.c#L1212-L1221>。

```c
void cleanup_message(struct strbuf *msgbuf,
	enum commit_msg_cleanup_mode cleanup_mode, int verbose)
{
	if (verbose || /* Truncate the message just before the diff, if any. */
	    cleanup_mode == COMMIT_MSG_CLEANUP_SCISSORS)
		strbuf_setlen(msgbuf, wt_status_locate_end(msgbuf->buf, msgbuf->len));
	if (cleanup_mode != COMMIT_MSG_CLEANUP_NONE)
		strbuf_stripspace(msgbuf,
		  cleanup_mode == COMMIT_MSG_CLEANUP_ALL ? comment_line_str : NULL);
}
```

- **scissors 切り捨て**は、`verbose` が真(`-v` / `commit.verbose`)のとき、または cleanup が
  `scissors` のときに行う。`verbose` は cleanup 方式と無関係で、エディターの有無とも無関係である。
  実測では、`git commit -v -m "<本文に scissors 行を含む>"` も
  `git -c commit.verbose=true commit -m ...` も、scissors 行以降が最終メッセージから消えた。
  `-v` なしの `-m` では、同じ scissors 行がそのまま残った。
  `--cleanup=scissors -m ...` でも、エディターを使わないため `whitespace` に落ち、残った(実測)。
- **コメント行の除去**は cleanup が `strip` のときだけ行う。
  `whitespace` ではコメント行が最終メッセージに残る。`verbatim` では何も変更しない。
- したがって、フックの入力と最終メッセージの差は次のとおりである(ソースからの推論を含む)。

1. エディター使用 + `strip`(エディターありの既定)
   - フックのファイル: コメント行を含む。
   - 最終メッセージ: コメント行と空白の過剰分が除かれる。
2. エディター使用 + `whitespace`
   - フックのファイル: 案内コメントを含む。
   - 最終メッセージ: コメントも残る(利用者が自分で消す前提)。
3. エディター使用 + `scissors`
   - フックのファイル: scissors 行以降を含む。
   - 最終メッセージ: scissors 行以降が切り捨てられる。
4. `-m` / `-F` + 既定
   - フックのファイル: 空白整理済みで、コメント行は残る。
   - 最終メッセージ: フックのファイルと同じ(`-v` なら scissors 以降が消える)。
5. `-m` / `-F` + `--cleanup=strip`
   - フックのファイル: 空白整理済みで、コメント行は残る。
   - 最終メッセージ: コメント行が除かれる(ソースからの推論)。
6. `--cleanup=verbatim`
   - フックのファイルも最終メッセージも、入力のまま。ただし `-v` が有効なら scissors 行以降は
     切り捨てられる。

- 上の 4(`-m` / `-F` の既定)と 5(`--cleanup=strip`)は、フックから見ると区別できない
  (ファイルの形が同じで、どちらも `GIT_EDITOR=:` で案内コメントもない)。
  区別できる材料は、`commit.cleanup` などの設定を自分で読んだ場合の予測だけで、
  コマンドラインの `--cleanup` と `-v` は見えない。

### 3.5 フックから cleanup 方式を判別できるか

- フックはファイルのパス(`COMMIT_EDITMSG` か `MERGE_MSG`)を受け取り、環境変数 `GIT_EDITOR` と
  フックを起動した時点の Git 設定を参照できる。ただし `GIT_EDITOR=:` は利用者も設定できるため、
  それだけではエディターの使用有無を確定できない。
- 設定は `git config` で読める(`commit.cleanup`、`commit.verbose`、`core.commentChar`、
  `core.commentString`)。ただしコマンドラインの `--cleanup` と `-v` による上書きは見えない。
  `git -c key=value commit` による設定の上書きがフック内の `git config` に見えるかは、
  今回は検証していない。
- 推論(未検証): エディター使用時の案内文には「ignored」「kept」の語と scissors 行が入るが、
  案内文はロケールで翻訳される(`_()` で包まれている、
  <https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L946-L959>)。
  案内文の文字列で方式を推定する実装は避けるべきである。scissors 行(`cut_line`)は翻訳されない
  定数だが、`whitespace` / `strip` では通常現れない。

## 4. コメント文字・scissors・cleanup の規則

### 4.1 cleanup 方式の決定

出典: <https://git-scm.com/docs/git-commit>(`--cleanup=<mode>`)と
<https://github.com/git/git/blob/v2.54.0/sequencer.c#L684-L701>(`get_cleanup_mode`)。

- 指定可能な値は `strip`、`whitespace`、`verbatim`、`scissors`、`default`。
- `default`(未指定、`commit.cleanup` も未設定): エディターを使うなら `strip`、使わないな
  ら `whitespace`。
- `scissors`: エディターを使うなら `scissors`、使わないなら `whitespace` になる。
- `strip` / `whitespace` / `verbatim` はエディターの有無に関係なくそのまま適用する。
- 優先順位: `--cleanup` があればそれを使い、なければ `commit.cleanup`
  (<https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L1830-L1834>)。
- エディターを使うかどうか: `-m`、`-F`、`-C`、`--fixup=<commit>`(`amend:` / `reword:` 以外)は
  使わない側に倒す。`-e` / `--no-edit` が最後に決める
  (<https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L1326>、L1409-L1414)。
- `-m` と `-F` は既定でエディターなし → `whitespace` になる。

### 4.2 scissors 行の厳密な形

出典: <https://github.com/git/git/blob/v2.54.0/wt-status.c#L42-L43>、L1124-L1139。

```c
static const char cut_line[] =
"------------------------ >8 ------------------------\n";
...
strbuf_addf(&pattern, "\n%s %s", comment_line_str, cut_line);
if (starts_with(s, pattern.buf + 1))
	len = 0;
else if ((p = strstr(s, pattern.buf))) { ... len = p - s + 1 ...
```

- 一致する行は `<コメント文字列> ------------------------ >8 ------------------------` に LF が続く
  行である。
  つまり**行頭から始まり、行全体が一致し、LF で終わる**必要がある。
  先頭の空白(インデント)、行末の余分な文字、CRLF(`\r\n`)は一致しない(ソースからの推論)。
  実測では、先頭に空白を 1 つ付けた scissors 行は切り捨てに使われなかった(`-v` 指定でも残った)。
- メッセージの最終行が LF で終わらない scissors 行は、パターンの末尾 `\n` に一致しない
  (ソースからの推論。ファイルの先頭行だけは `starts_with(s, pattern.buf + 1)` で別扱い)。
- 切り捨てるのは最初に一致した行の直前までで、一致した行自体と以降をすべて捨てる。
- コメント文字列はこの行にも含まれるため、`core.commentChar` / `core.commentString` に依存する。
  実測(`core.commentString=//`)では、案内文のコメント接頭辞が `//` になった。
  scissors 行も `// ------------------------ >8 ------------------------` になる(ソースからの推論)。
- `git commit -v` 以外にも、`git status`、`git commit --dry-run -v` などが
  `wt_status_add_cut_line` を呼んで scissors 行を足す。
  フックの入力に入るのは `git commit` の場合である。

### 4.3 コメント文字

出典: <https://git-scm.com/docs/git-config>(`core.commentChar` / `core.commentString`)、
<https://github.com/git/git/blob/v2.54.0/environment.c#L444-L462>。

- `core.commentChar` と `core.commentString` は互いの別名である。既定値は `#`。
- 複数文字(例: `//`)は **Git 2.45.0 以降**。2.45.0 より前の Git は `commentString` を無視し、
  1 つの ASCII バイトより長い `commentChar` を拒否する(git-config の説明そのまま)。
- 空文字列は拒否され、値に LF を含められない(`environment.c`)。
- コメント行の判定は「その文字列で**始まる行**」である。
  `strbuf_stripspace` の `starts_with(sb->buf + i, comment_prefix)`
  (<https://github.com/git/git/blob/v2.54.0/strbuf.c#L1085-L1113>)。
  先頭に空白がある行はコメントにならない。文字列として前置一致するため、
  コメント文字列が `#` のとき `#foo`(空白なし)もコメント行になる。
- コメント文字は `commit` と `tag` などの編集コマンドに適用される(git-config の説明)。
  `git stripspace -s` も `core.commentChar` を使う(<https://git-scm.com/docs/git-stripspace>)。
- 設定の読み方: Git は設定を `local` と `global` などから重ねて読む。
  linter が単純に `.git/config` だけを読むと、`global` / `system` / `include` を取りこぼす。
  両方が設定されたときの優先は「後に読んだ方」である
  (`environment.c` では後勝ち。両名とも同じ変数を書き換える)。
  そのため、実装するなら `git config` で両方の名前を設定順に取得して最後の値を使う必要がある。

### 4.4 `core.commentChar=auto`

- git-config の記述(出典 <https://git-scm.com/docs/git-config>):
  「If set to "auto", git-commit will select a character that is not the beginning character of any
  line in existing commit messages. Support for this value is deprecated and will be removed in
  Git 3.0」。理由として、テンプレート・cherry-pick / merge / rebase / revert の
  コメント、`prepare-commit-msg` フックのコメント、rebase の `fixup` / `squash`、`git notes` と
  両立しないことを挙げている。
- 実装: 値の比較は `strcasecmp`(大文字小文字を区別しない)で、`auto` のとき `comment_line_str` は
  一旦 `#` になり、`auto_comment_line_char` が立つ。`WITH_BREAKING_CHANGES` を付けた
  ビルド(Git 3.0 相当)ではこの分岐が無くなり、設定すると
  「Support for '%s=auto' has been removed in Git 3.0」で `die` する
  (<https://github.com/git/git/blob/v2.54.0/environment.c#L444-L462>、
  <https://github.com/git/git/blob/v2.54.0/config.c#L2190-L2225>)。
  Git 2.54.0 では警告を 1 回出す(実測で
  `warning: Support for 'core.commentChar=auto' is deprecated and will be removed in Git 3.0`)。
- 文字の選び方(<https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L700-L736>
  `adjust_comment_line_char`): 候補は `"#;@!$%^&|:"` の順。メッセージ中で
  **行頭に現れない**最初の候補を採用する(末尾の `Conflicts:` などのコメント部分は
  `ignored_log_message_bytes` で除外)。すべて使われていれば `die` する。
  実測では、本文に `# x` の行があるメッセージで `;` が選ばれ、
  案内文も `; Please enter ...` になった。
- 重要: 選ばれた文字は**メモリ上の値**で、設定にも環境変数にもフックの引数にも出ない。
  `git config core.commentChar` は `auto` を返す。フックから確実に知る方法はない。
  推論(未検証の実装案): ファイル末尾のコメント塊で、全行が候補文字のいずれかで始まる
  ブロックの接頭辞を推定する。`auto` を使う利用者は少数で、Git 3.0 で消えるため、
  対応コストに見合うかは別問題である。

## 5. 空メッセージ

- `git commit` は cleanup の後に、メッセージが空(空白と `Signed-off-by` 行だけ)なら中止する。
  `--allow-empty-message` で許可できる。
  出典: <https://git-scm.com/docs/git-commit>(`--allow-empty-message`)、
  <https://github.com/git/git/blob/v2.54.0/sequencer.c#L1223-L1232>(`message_is_empty`、
  コメント「contains only whitespace and Signed-off-by lines」)、
  <https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L1904-L1910>。
- `verbatim` のときは、長さが 0 でなければ空とみなさない(`message_is_empty` の
  `cleanup_mode == COMMIT_MSG_CLEANUP_NONE && sb->len`)。
- 判定は `commit-msg` の**後**で行うため、フックは空のファイルや、コメントだけのファイルを受け取る。
  実測では `git commit -m ""` でフックが呼ばれ(ファイルは空)、そのあと Git が
  `Aborting commit due to empty commit message.` で中止した。
  linter が空メッセージを `header-format` エラーにするのは、Git の中止と結果が一致する。
  ただし `strip` のエディター入力でコメントしかないファイルは、linter では空でなくコメント行を
  Header とみなして別のエラーにする(現行仕様)。Git の判定に揃えるなら、
  「cleanup 後に空」を空メッセージとして扱う案がある。
- テンプレート(`--template` / `commit.template`)を編集せずに保存すると中止する
  (`template_untouched`)。これは linter が見られない状態(テンプレートの内容)に依存する。
- `--fixup=amend:<commit>` で作る `amend!` コミットは、件名の後ろが空のとき中止する
  (<https://github.com/git/git/blob/v2.54.0/builtin/commit.c#L1917-L1925>)。

## 6. エンコーディング

- 出典: <https://git-scm.com/docs/git-commit> の文字エンコーディングに関する節
  (「Git is to some extent character encoding agnostic.」で始まる)。
  - コミットログは通常 UTF-8 だが、ISO-8859-x や CP125x などの拡張 ASCII も扱える。
    UTF-16 / UTF-32、EBCDIC、CJK のマルチバイト(GBK、Shift-JIS、Big5、EUC-x、CP9xx)は**扱えない**。
  - `git commit` と `git commit-tree` は、メッセージが妥当な UTF-8 に見えない場合に**警告**する。
    プロジェクトが別のエンコーディングを使うなら `i18n.commitEncoding` に設定する。
    設定するとコミットオブジェクトの `encoding` ヘッダーにその値が入る。
    ヘッダーがなければ UTF-8 とみなす。
  - `git log` などは `encoding` ヘッダーを見て、`i18n.logOutputEncoding`
    (未設定なら `i18n.commitEncoding`)へ変換する。
- 実装: 警告は `encoding_is_utf8` が真のときだけ出し、コミットは続行する
  (<https://github.com/git/git/blob/v2.54.0/commit.c#L1822>)。**Git は不正な UTF-8 を拒否しない**。
- 影響(ソースからの推論): 現行の linter は不正な UTF-8 を入力エラー(終了コード 2)にする。
  `i18n.commitEncoding` が UTF-8 以外のリポジトリでは、フックのファイルがその
  エンコーディングのままになるため、`commit-msg-lint` がフックで常にエラーになりうる。
  Git の仕様を優先するなら、`i18n.commitEncoding` を読んで変換する案と、
  UTF-8 以外を明示的に未対応として案内する案がある。後者は現行仕様に近い。
- `i18n.commitEncoding` の既定値は `utf-8`
  (出典: <https://git-scm.com/docs/git-config>)。

## 7. 自動生成メッセージと trailers

### 7.1 自動生成・特殊な件名

- `git commit --fixup=<commit>` は件名が `fixup! <元の件名>` のコミットを作る。
  `--fixup=amend:<commit>` は `amend! `、`--squash=<commit>` は `squash! ` で始まる
  (<https://git-scm.com/docs/git-commit>)。`git rebase --autosquash` は件名がこれらで
  始まるとき、残りを対象コミットの指定として扱う
  (<https://git-scm.com/docs/git-rebase>)。ソースの判定は
  <https://github.com/git/git/blob/v2.54.0/sequencer.c#L6646-L6648>
  (`fixup! `、`amend! `、`squash! `)。
  Conventional Commits の Header 形式にはならないため、現行の `header-format` で必ず失敗する。
  該当 Header の扱い(スキップか緩和か)は設計判断である。
- `git revert` は件名を `Revert "<元の件名>"`、本文に `This reverts commit <hash>` を入れる。
  すでに `Revert "..."` を取り消すと `Reapply "..."` になる
  (<https://github.com/git/git/blob/v2.54.0/sequencer.c#L5620-L5635>)。
  これも Conventional Commits の形式ではない。
- `git cherry-pick -x` は `(cherry picked from commit ` で始まる行を足す
  (<https://github.com/git/git/blob/v2.54.0/sequencer.c#L64>)。
- `git merge` の既定メッセージ(`Merge branch ...`)は `MERGE_MSG` に書かれ、
  `commit-msg` に渡される(3.1 節)。
- これらのメッセージは編集せずにフックを通らない場合もある(3.1 節、シーケンサー)。
- 未検証: rebase の `squash` が作るまとめメッセージ内のコメント(`# This is a combination of ...`)の
  文言とコメント文字の扱い。git-config の `auto` の説明が両立しないと述べる事実のみ確認した。

### 7.2 trailers

- `git interpret-trailers` の trailer ブロックは、メッセージの末尾にあり、(i) 全行が trailer か、
  (ii) Git が生成した trailer か設定済みの trailer を 1 つ以上含み、かつ **25% 以上**が trailer の
  行のまとまりである。メッセージは入力の末尾か、`---`(後ろが空白か行末)の行より前の最後の
  非空白行までである。
  出典: <https://git-scm.com/docs/git-interpret-trailers>(「A trailer block is a group of one or
  more lines that (i) is all trailers, or (ii) contains at least one Git-generated or
  user-configured trailer and consists of at least 25% trailers.」)。
- 区切り文字は既定で `:` のみ(`trailer.separators` で変更可、コマンドラインの `=` は例外)。
  同ドキュメントの `trailer.separators` 節。
  現行の linter が有効な trailer とみなす ` #`(スペースとハッシュ。`Refs #123`)は
  Git の既定の trailer 区切りではない。`BREAKING CHANGE` の特別扱いも Git にはない
  (Conventional Commits 由来)。`docs/message-format.md` は自身を
  「Git trailer の一般的なパーサーではない」と明記しているため矛盾ではないが、
  Git の trailer と比較する際の差分として記録する。
- `git interpret-trailers` は**コメント行を無視する**(`core.commentString`)。
  同ドキュメントは「This is for use with the prepare-commit-msg and commit-msg hooks」と書いている。
  つまり Git 自身も、フックでコメント付きのファイルから trailer を読む用途を想定している。
- `---` の行は、format-patch の出力などで「メッセージの終わり」とみなされる(既定。
  `--no-divider` で無効)。コミットメッセージ中の `---` 以降は、`git interpret-trailers` では
  対象外になる。現行の linter は `---` を特別扱いしない。
- `git commit --trailer` と `-s` は、フックを呼ぶ前にファイルへ trailer を追加する(3.2 節)。
  このため `commit-msg` で検査する Footer には、利用者が書かなかった trailer も含まれる。
  `trailer.*` 設定(`ifexists`、`where` など)で追加位置や重複の扱いが変わる
  (<https://git-scm.com/docs/git-commit> の `--trailer`)。
- 未検証: trailer 値の継続行(行頭が空白の行)の扱いと、`Co-authored-by` 等の生成。

## 8. 過去の調査メモの推奨に対する確認と訂正

### 8.1 gitlint 調査(`2026-11-09-gitlint-parser-design.md`)3.3 節

- 「シザー行は行全体の完全一致で探し、インデント付きは無視」: **確認**。
  - 4.2 節。実測でも空白付きは残った。
  - 補足: **LF 終端**が必要で、CRLF の行は一致しない。
- 「シザー行は `commit --verbose` / `cleanup=scissors` と同じ文字列」: **確認(補足あり)**。
  - 切り捨ては `-v` / `commit.verbose` でも発生する。
  - `cleanup=scissors` が `whitespace` に落ちる `-m` でも、`-v` があれば切り捨てられる
    (3.4 節の実測)。
- 「コメント行は `startswith` で除去し、commit-msg フックは cleanup 前のファイルを
  受け取る」: **訂正**。
  - フックのファイルは cleanup 前だが、`-m` / `-F` の既定(`whitespace`)はコメントを除去しない。
    コメントはコミットに残るため、無条件の除去は誤検査の原因になる(3.2〜3.4 節)。
- 「先頭・末尾の空行を除去しない」: **確認(補足あり)**。
  - `-m` / `-F` では、フックが空白整理済みのファイルを見る(実測)。
  - 通常ファイルや標準入力では未整理のままである。
- 「複数文字の `core.commentChar` は前置一致で動く」: **確認**。
  - 2.45.0 以降(4.3 節)。「行頭のみ」の条件は保つこと。
- 「`auto` は本文で未使用の文字を選ぶので、Git と同じ規則で解決すべき」: **訂正**。
  - 選んだ文字はフックから得られない。
  - `auto` は非推奨で、Git 3.0 で削除される。規則を再現しても結果を確定できない(4.4 節)。

### 8.2 commitlint 調査(`2026-11-09-commitlint-parser-design.md`)3.3 節と引き継ぎ事項

- 「`commentChar` 指定時にシザー行以降を切り捨て、コメント行を除去する」は、
  Git の `strip` + `scissors` を 1 つに束ねた処理であり、cleanup 方式に応じた分岐はない。
  Git では方式ごとに異なる(4.1 節)。
- 「前処理(コメント文字、シザー行、改行コード)の仕様。`core.commentChar` に追従する点や、
  シザー行の厳密な文字列は Git 互換性のために合わせるべき」は、**文字列の厳密性は確認**した。
  「Git 互換性のために合わせるべき」かは、入力経路で変わる(9 章)。
- 「`auto` は Git と同じ規則(本文に現れない候補文字を選ぶ)で解決し、複数文字のコメント文字列も扱う」
  (gitlint 調査 6 章)は、`auto` の部分を訂正する(8.1 節)。複数文字の部分は確認した。

## 9. 入力経路ごとの整理(行番号の扱いを含む)

- フック(`--edit "$1"`)
  - 入力の性質: `COMMIT_EDITMSG` か `MERGE_MSG`。cleanup 前。`GIT_EDITOR` が設定される。
  - コメント・scissors: 方式は判別できない(3.5 節)。
    `GIT_EDITOR` が `:` でなければ編集用ファイルとみなせる。
  - 行番号: ファイル上の行番号が正しい。利用者が開くエディターのバッファと一致する。
- 通常ファイル(`--edit`)
  - 入力の性質: 任意のファイルで、出所は不明。
  - コメント・scissors: Git の作業ファイルとは限らない。
    原則は検査のみとし、除去は明示指定にする案が自然である。
  - 行番号: ファイル上の行番号が正しい。
- 標準入力
  - 入力の性質: `git log -1 --format=%B`、`git show`、パイプ入力など。
  - コメント・scissors: `git log` の出力のように、cleanup 後の最終メッセージのことが多い。
    その場合、コメント行は本物の本文である。
  - 行番号: `<stdin>` の入力上の行番号。Git の元ファイルとは無関係である。
- Web デモ
  - 入力の性質: 貼り付けた文字列。Git の設定も環境変数もない。
  - コメント・scissors: コメント文字を判定する材料がない。現状の「そのまま検査」は一貫している。
  - 行番号: 貼り付けた文字列上の行番号。

- 診断の行番号は、**除去前のファイル上の行番号**で報告するのが望ましい。
  `TODO.md` の想定どおり、フックの入力と標準入力・通常ファイルを区別するなら、
  除去した行を数えたまま元の行番号を保つ必要がある(過去調査の gitlint が行番号をずらす問題を
  避けるため)。ただし Git が cleanup で行を削る(先頭の空行の除去、連続空行の圧縮)ため、
  **最終メッセージ上の行番号とは一致しなくなる**。`git commit` の結果の行ではなく、
  利用者が編集したファイルの行を指すと明示する必要がある。
- 現行の `docs/message-format.md` は「Line numbers in diagnostics are one-based after CRLF
  normalization」と定義し、除去を行わない。除去を導入するなら、この定義の変更が必要になる。
- 範囲の考え方: scissors 行より前を残し、コメント行を除去し、その後で
  Header / Body / Footer に分類する順で行う場合、除去した行は分類に含めず、診断にも出さない。
  ただし除去した行にある制御文字や
  幅の問題は、Git が除去するなら無視でよい。`whitespace` のようにコメント行が残る状況では、
  逆に検査対象に含める必要がある。

## 10. 実装への留意点(検討材料)

決定ではなく、将来の検討で外せない条件を挙げる。

1. **経路ごとに前処理を分ける**。フックの入力に「Git の cleanup を再現する前処理」を
   当てるのは意味があるが、標準入力・Web デモの入力に同じ前処理を暗黙に当てると、
   本物の `#` 行を落とす。前処理は明示的な入力モード(例: フック用)にとどめる案が安全である。
2. **cleanup 方式はフックから確定できない**。`GIT_EDITOR` が `:` でないときだけ
   `strip` 相当とみなす近似は、`commit.cleanup=whitespace` や `--cleanup=scissors` の
   利用者で外れる。外れたとき、誤ってコメントを除去するのと、誤ってコメントを検査するのとで、
   どちらを許容するかを決める必要がある(README の警告・エラーの区別が使える可能性がある)。
3. **scissors は `-v` / `commit.verbose` でも効く**。`GIT_EDITOR=:` の `-m` でも、
   本文に scissors 行があり `-v` が有効なら最終メッセージから消える(実測)。
   scissors の切り捨てと、コメント行の除去は、別々のスイッチにするのが Git の構造に合う。
4. **scissors 行の判定は厳密にする**。行頭、`<コメント文字列> ` の接頭辞、
   固定の文字列、LF 終端。CRLF を LF に正規化してから判定するなら、Git の挙動
   (CRLF では一致しない)より緩くなる点を許容するか決める。
5. **コメント文字は設定から読む**。`core.commentString` と `core.commentChar` は設定順に読み、
   後勝ちとする。
   2.45.0 未満では複数文字がない。`auto` は検出して警告し、確定できないと明示するのが誠実である。
   Web デモと標準入力には設定がないため、既定の `#` か、明示指定(設定ファイルの項目)になる。
6. **`-m` / `-F` の入力は空白整理済み**で、コメント行が残る。フックの入力を「エディター由来」と
   決めつけない。`GIT_EDITOR=:` は「エディターなし」の強い手がかりだが、利用者が自分で設定した場合を
   区別できない。
7. **フックが動かない経路がある**(cherry-pick、revert、編集なしの rebase)。
   これらを検査したいなら、フックではなく履歴の検査(`TODO.md` の別項目)が必要になる。
8. **エンコーディング**: 不正な UTF-8 を入力エラーにする現行仕様は Git より厳しい。
   `i18n.commitEncoding` を使うリポジトリで、フックが常に終了コード 2 になる点を
   ドキュメントで明示するか、変換を検討する。
9. **自動生成の件名**(`fixup!`、`amend!`、`squash!`、`Revert "`、`Merge `)は、
   Conventional Commits の Header 形式と衝突する。除外の設計(ルール単位か、入力単位か)が必要になる。
10. **trailer の定義**は Git と Conventional Commits で異なる。Git の trailer(`:` 区切り、
    25% 規則、`---` 以降の除外、コメント行の無視)と、現行の Footer 判定の差を
    `TODO.md` の「Message compatibility」の評価項目で扱える。
11. **空メッセージ**は Git が cleanup 後に判定して中止する。フックが先に空ファイルや
    コメントのみのファイルを受け取る点を、診断の文言で補う案がある。

## 11. 未確認事項と限界

- 実測は Git 2.54.0 / macOS の 1 環境のみ。他のバージョンでは 3.3〜4.4 節の挙動が異なりうる
  (とくに `auto` の警告・削除と、2.45.0 未満のコメント文字列)。
- `git commit --amend --no-edit`、`-C`、`-c`、`git merge` の実測はしていない(ソースの読解のみ)。
- `GIT_CONFIG_PARAMETERS` 経由の `-c` 設定がフックから見えるかは未検証。
- `include` / `includeIf` を含む設定の読み取りは、`git config` に任せる前提で、独自実装した場合の
  差は調べていない。
- trailer 値の継続行、`trailer.*` 設定の詳細、`git notes` / `git tag` のコメント処理は未調査。
- Git 2.54 の設定ベースのフック(`hook.<name>.command`)は git-config に項目があることだけを確認した。
  フックの引数・環境変数が従来の `$GIT_DIR/hooks/commit-msg` と同一かは未検証。
- 作業用の使い捨てリポジトリでの実測は、調査後に削除した。

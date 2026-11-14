# Git コミットメッセージ入力の分類

調査日: 2026-11-14。入力経路とデータの違いを整理する。実装方針は決めない。

詳細なフック実測は[先行調査](2026-11-13-git-commit-message-input.md)を、検討中の実装方針は[設計
案](../design/commit-message-parsing-and-linting.md)を参照する。Git 固有の実装挙動は、特記のない限り
Git v2.54.0 のソースに基づく。

## メッセージの生成元

- **エディター入力**: `git commit` が用意するファイルには、手書きの文、テンプレート、既存メッセージ
  のほか、Git の案内・status・`-v` の差分が含まれ得る。`--no-status` で status 表示を抑制できる。
  出典: [git
  commit](https://git-scm.com/docs/git-commit)、[githooks](https://git-scm.com/docs/githooks)
- **コマンド引数**: `git commit -m` は文字列を受け取る。複数回の `-m` は別段落として連結す
  る。`-F <file>` はファイルから読み、`-F -` は標準入力から読む。通常はエディターを開かない
  が、`-e` で編集できる。
  出典: [git commit](https://git-scm.com/docs/git-commit)
- **既存メッセージの再利用**: `-C` は既存コミットの文を再利用し、`-c` は編集す
  る。`--amend`、`--fixup`、`--squash` も異なる形式の件名を生み得る。
  出典: [git commit](https://git-scm.com/docs/git-commit)
- **Git が生成する文**: merge、revert、cherry-pick などが作るメッセージも対象になり得
  る。`Merge ...`、`Revert ...`、`fixup! ...` のように、通常の Conventional Commits 形式でない場合が
  ある。
  出典: [git merge](https://git-scm.com/docs/git-merge)、[git
  revert](https://git-scm.com/docs/git-revert)
- **パッチメール**: `git am` はメールからメッセージを作る。`applypatch-msg` フックが提案メッセージを
  受け取り、Git 付属の既定フックが有効なら `commit-msg` も呼び得る。メールの `--scissors` は、commit
  の scissors とは別の処理である。
  出典: [githooks](https://git-scm.com/docs/githooks#_applypatch_msg)、[git
  am](https://git-scm.com/docs/git-am)
- **完成済みコミット**: `git log` や `git show` の出力を検査する経路もある。表示時に文字コード変換や
  整形が起こり得るため、出力はコミットオブジェクト内の生データと常に同じではない。
  出典: [git log](https://git-scm.com/docs/git-log)、[pretty
  formats](https://git-scm.com/docs/pretty-formats)
- **任意のファイルや文字列**: 標準入力、ファイル、貼り付けの由来は内容だけでは分からない。現行 CLI
  は標準入力か `--edit` ファイルをそのまま検査する。
  出典: [CLI 仕様](../cli.md)
- **plumbing / サーバー側**: `git commit-tree` はメッセージからコミットを作る。
  通常の `commit-msg` フックは通らない。
  `pre-receive` の標準入力はメッセージではなく、更新対象の ref とオブジェクト ID である。
  出典: [git commit-tree](https://git-scm.com/docs/git-commit-tree)
  と [githooks](https://git-scm.com/docs/githooks#_pre-receive)

## どの段階の内容を読むか

`git commit` では、選んだ入力から Git が作業ファイルを用意し、`prepare-commit-msg`、必要ならエディ
ター、`commit-msg` の順に処理した後、Git が cleanup してコミットを作る。フックはメッセージファイルを
変更できる。

したがって、次の三つは同じとは限らない。

1. `-m` / `-F` などで渡した元の入力
2. `commit-msg` フックが読むファイル
3. 最終的にコミットに保存されるメッセージ

`commit-msg` が読むのは cleanup 前のファイルである。ただし `-m` / `-F` の元データそのものとも限らな
い。例えば `-s` / `--trailer` の追加はフックより前に反映される。

## コメント、cleanup、scissors

cleanup の方式は `--cleanup`、`commit.cleanup`、エディター使用の有無などに依存する。主要な方式は次の
とおりである。

- `strip`: 空白を整理し、コメント行を除く。
- `whitespace`: 空白を整理するがコメント行を残す。
- `verbatim`: cleanup による整理をしない。
- `scissors`: 編集時は scissors 行以降を切り、残りを `whitespace` 相当に整理する。編集しない場合
  は `whitespace` 相当になる。
- `default`: 通常、編集時は `strip`、編集しない場合は `whitespace`。

コメント接頭辞の既定値は `#` だが、`core.commentChar` / `core.commentString` で変更できる。Git 2.45
以降は `//` のような複数文字も使える。よって、コメント行を常に `#` だけで判断するのは不十分である。

scissors 行の既定形は次のとおりである。

```text
# ------------------------ >8 ------------------------
```

編集時に `--cleanup=scissors` を使う場合や、`-v` / `commit.verbose` が有効な場合など、行以降が最終
メッセージから除かれることがある。一方、同じ行が常に除かれるわけではない。特に `-m` / `-F` をエディ
ターなしで使う既定では `#` 行も保存され得る。

フックの引数は実効 cleanup 方式や `-v` の有無を伝えない。そのため、フック入力だけから最終結果を常に
復元することはできない。詳細な条件と実測は[先行調査](2026-11-13-git-commit-message-input.md)を参照す
る。

## 設計上の含意

コメントや差分をフック用に除外すれば、編集補助を本文と誤認するケースを減らせる。一方、Git がその行を
実際に保存する操作では違反を見逃し得る。原文を検査すれば見逃しは減るが、Git が後で除く行を理由に拒否
する可能性がある。どちらを採るかは、入力経路や利用目的に応じた別の設計判断である。

現行 CLI は UTF-8 を要求するが、Git のコミットメッセージは別のエンコーディングを使う構成もあり得る。
また、`--no-verify` や一部の操作では `commit-msg` が呼ばれない。フックだけで全経路を検査できるわけで
はない。

これらの違いは、README の[設計方針](../../README.md#design-philosophy)にある表示幅の測定と、警告・エ
ラーの使い分けを置き換えるものではない。

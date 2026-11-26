# Issue トラッカー: GitHub

[English](issue-tracker.md)

このリポジトリの issue と仕様は GitHub Issues に置く。以下のローカル成果物のルールが適用される
場合を除き、すべての操作に `gh` CLI を使う。

## エンジニアリングスキルのローカル成果物

`/to-spec` と `/to-tickets` は成果物を `.scratch/` 以下のローカル Markdown ファイルとして保存する。
これらのファイルはコミットせず、既定では GitHub に公開しない。

- `/to-spec`: 仕様を `.scratch/<feature-slug>/spec.md` に保存する。
- `/to-tickets`: チケットごとに `.scratch/<feature-slug>/issues/<NN>-<slug>.md` 以下に
  ファイルを保存し、依存順（ブロッカーが先）に `01` から番号を振る。
  各チケットの `Blocked by` フィールドに、そのチケットをブロックする依存関係を記載する。

ユーザーが明示的に成果物の公開を求めた場合は、GitHub Issues と以下の規約を使う。

## 規約

- **issue の作成**: `gh issue create --title "..." --body "..."`。
  複数行の本文にはヒアドキュメントを使う。
- **issue の読み取り**: `gh issue view <number> --json number,title,body,labels,comments`。
- **issue の一覧表示**: `gh issue list --state open --json
  number,title,body,labels,comments --jq '[.[] | {number, title, body, labels:
  [.labels[].name], comments: [.comments[].body]}]'` に適切な `--label` と `--state`
  フィルターを付ける。
- **issue を親のサブ issue にする**: `gh issue create --parent <parent> ...`、
  または作成後に `gh issue edit <parent> --add-sub-issue <child>`（`gh` 2.94 以降）。
  古い `gh` の場合は
  `gh api --method POST repos/<owner>/<repo>/issues/<parent>/sub_issues -F
  sub_issue_id=<child-db-id>`（データベース ID。以下の **ブロック** を参照）。
  サブ issue が使えない場合は、子の本文の先頭に `Part of #<parent>` を記載する。
- **ブロック**: GitHub の**ネイティブな issue 依存関係**を正規の UI に表示される表現とする。
  `gh issue create --blocked-by <n>,<n> ...` または
  `gh issue edit <child> --add-blocked-by <blocker>`（`gh` 2.94 以降）で依存関係を追加する。
  古い `gh` の場合は `gh api --method POST
  repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F
  issue_id=<blocker-db-id>` を使う。`<blocker-db-id>` はブロッカーの数値の
  **データベース ID**（`gh api repos/<owner>/<repo>/issues/<n> --jq .id` で取得し、
  `#number` や `node_id` ではない）。依存関係を利用できない場合は、代わりに子の本文の先頭に
  `Blocked by: #<n>, #<n>` という行を記載する。すべてのブロッカーがクローズされると、
  チケットのブロックは解除される。
- **未クローズのブロッカー数の取得**:
  `gh api repos/<owner>/<repo>/issues/<n> --jq
  .issue_dependencies_summary.blocked_by`（未クローズのブロッカーのみ。0 なら着手可能）。
  `gh issue view <n> --json blockedBy` はクローズ済みのブロッカーも列挙するため、
  `--jq '[.blockedBy.nodes[] | select(.state == "OPEN")] | length'` で絞り込む。
- **issue へのコメント**: `gh issue comment <number> --body "..."`。
- **ラベルの追加・削除**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`。
- **クローズ**: `gh issue comment <number> --body "..."` で説明を投稿してから
  `gh issue close <number>` を実行する。すでにクローズされた issue に対しては、
  `gh issue close --comment` はコメントを捨てるが、終了コードは 0 のままである。

リポジトリは `git remote -v` から判断する。クローン内で実行すると `gh` は自動的に判断する。

issue はユーザー自身のリポジトリにのみ作成する。ユーザーが所有していない限り、
`mattpocock/skills`（これらのスキルの出所であるリポジトリ）には決して作成しない。
フォークでは `gh` の既定対象が上流の親になる場合があるため、セッションで最初に
`gh issue create` を実行する前に `gh repo view --json nameWithOwner` で対象を確認し、
ユーザーのリポジトリでなければユーザーに確認する。

## トリアージ対象としてのプルリクエスト

**PRs as a request surface: no.** _（このリポジトリで外部 PR を機能リクエストとして
扱う場合は `yes` に設定する。`/triage` は英語版のこのフラグを読む。）_

`yes` に設定すると、PR は issue と同じラベルと状態を使用し、対応する `gh pr` コマンドで扱う。

- **PR の読み取り**: `gh pr view <number>`、次に `gh pr view <number> --comments`
  （TTY 以外では `--comments` は本文を表示せずコメントのみを表示する）、
  `gh pr diff <number>` で差分を確認する。
- **トリアージ対象の外部 PR の一覧表示**:
  `gh api --paginate 'repos/{owner}/{repo}/pulls?state=open' --jq
  '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") |
  not) | {number, title, author: .user.login, author_association, labels:
  [.labels[].name]}'`。
- **コメント・ラベル付け・クローズ**: `gh pr comment`、
  `gh pr edit --add-label` / `--remove-label`、`gh pr close`。

GitHub では issue と PR の番号空間が共通なので、単なる `#42` はどちらの場合もある。
`gh pr view 42` で確認し、失敗したら `gh issue view 42` を使う。

## スキルが「issue トラッカーに公開する」と指示した場合

GitHub issue を作成する。ただし `/to-spec` と `/to-tickets` は、ユーザーが明示的に公開を
求めない限り、上記のローカル成果物のルールに従う。

## スキルが「関連するチケットを取得する」と指示した場合

上記の **issue の読み取り** に従って読む。

## Wayfinding 操作

`/wayfinder` が使用する。**マップ**は、チケットとしての**子** issue を持つ単一の issue である。

- **マップ**: `wayfinder:map` ラベルが付き、Notes / Decisions-so-far / Fog の本文を持つ単一の
  issue。`gh issue create --label wayfinder:map`。
- **子チケット**: GitHub のサブ issue としてマップにリンクされた issue（**issue を親のサブ issue にする**
  を参照）。サブ issue が有効でない場合は、マップ本文のタスクリストに子を追加し、
  子の本文の先頭に `Part of #<map>` を記載する。ラベルは `wayfinder:<type>`
  （`research` / `prototype` / `grilling` / `task`）。着手されたチケットは担当開発者に割り当てる。
- **ブロック**: 規約の **ブロック** を参照。
- **フロンティアの検索**: マップの未クローズの子を一覧表示し
  （`gh issue list --state open` をマップのサブ issue / タスクリストに絞る）、
  未クローズのブロッカー（**未クローズのブロッカー数の取得**、または `Blocked by` 行の
  未クローズ issue を参照）があるものや担当者がいるものを除外する。マップ順で最初のものを選ぶ。
- **着手**: `gh issue edit <n> --add-assignee @me`。セッション最初の書き込みとなる。
- **解決**: `gh issue comment <n> --body "<answer>"`、次に `gh issue close <n>` を実行し、
  マップの Decisions-so-far にコンテキストへのポインター（gist + リンク）を追記する。

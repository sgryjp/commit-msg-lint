# コミットメッセージ流儀と既定設定の適合性

- 調査日: 2026-11-20。目的は、Conventional Commits を対象とする本ツールが、実際の流儀をどの程度拒否するかを見積もることである。
- 対象: リポジトリの `30950f41d06b9ad4ec87f0b7d34f0e1723f79bcc`、CLI `0.0.2`。
  外部資料は公式仕様、各プロジェクトの公式ガイドと公開コミット、Git・GitHub の公式資料を
  `ketch` で確認した。変化するガイドは調査日時点の内容である。
- 実測は `cargo build` 相当のビルド済み CLI、最小設定 `{"version":1}`、通常モードを使用した。
  リポジトリの `commit-msg-lint.jsonc` は Type をさらに絞るため使わない。入力は stdin、
  一時ディレクトリで実行し、明示的に最小設定を指定した。終了コード 1 を失敗とし、
  warning のみは成功とした。`--strict`、設定変更、Git フックの動作は別条件である。

## 要約

- Conventional Commits そのものに適合するだけでは、既定設定の成功は保証されない。
  任意の Type、Type の大小文字、文末のピリオド、72 表示桁を超える本文・Footer は衝突し得る。
  公式仕様の実例も Footer 幅で失敗する。これは単に Conventional Commits 以外を
  除外しているという話ではない[^cc][^format][^config]。
- Angular 形式の典型的な Header は適合するが、Angular・commitlint・conventional-changelog の
  default branch 最新各 100 コミットを実測すると、それぞれ 55、86、68 件が失敗した。
  特に長い本文や Footer が多く、commitlint の Footer 幅エラー 75 行中 74 行が
  `Co-authored-by:` であった。これは選んだ履歴の観測であり、全利用者やコミット時点における
  拒否率ではない。
- Drupal Core の `task:` は Conventional Commits 形式の実在する例だが、既定 Type に含まれず失敗する。
  Go・Linux kernel の `package/subsystem: ...` や Django の `Fixed ...` は代表的な別プロジェクトの
  流儀だが、Type か Header の意味自体が異なる。これらを対象に含めるかは Conventional Commits を
  掲げる製品の境界の問題であり、仕様違反のメッセージを拒否すること自体を誤りとは
  断定できない[^drupal][^go][^kernel][^django]。
- Git の典型的な autosquash・revert・merge は既定で検査全体から除外され、この範囲では既に回避される。
  一方、GitHub の squash merge 件名は PR タイトル等によって変わり、全件が生成メッセージとして
  除外されるわけではない。除外は生成元の証明でも対象本文の検査でもない[^git-commit][^github-merge][^config]。

## 1. 判定の基準と本ツールの境界

本ツールは Conventional Commits を検査する CLI である。既定では Header が
`<type>[(<scope>)][!]: <subject>` にパースされる必要があり、Type は既定の 11 種に限られ、
小文字を要求し、Subject 末尾の ASCII ピリオドを禁止する。Header の 50 表示桁超は warning、
72 超は error、Body と Footer の 72 超は error である。Footer 判定は Git trailer の完全なパーサーではなく、
行位置や `:`・`#` を使うヒューリスティックである[^format][^config]。

Conventional Commits 1.0.0 は `feat`・`fix` 以外の Type を許し、Type 等の情報を
大文字小文字で区別しないことを要求する。FAQ は Type のどの大文字小文字も許す。
仕様は Body を自由形式とし、表示桁数や Subject 末尾のピリオドを禁止していない。
例えば仕様の `docs: update ref docs.` は既定の `subject-full-stop` に反する[^cc]。

以下の「失敗」は上記条件での実測であって、当該流儀全体の拒否率を意味しない。一次資料からの直引用と、形を説明するための合成例を区別する。

## 2. 仕様・各プロジェクトの流儀

### Conventional Commits 1.0.0

公式の `docs: correct spelling of CHANGELOG` は成功した。同じ公式ページの次の完全な例は、
Header の 58 桁が warning、Footer の 90 桁が `footer-max-width` error となり失敗した[^cc]。

```text
feat: allow provided config object to extend other configs

BREAKING CHANGE: `extends` key in config file is now used for extending other config files
```

仕様が認める形を示す合成例 `FIX: prevent racing of requests` は `type-case`、
`task: update dependencies` は `type-enum`、公式の `docs: update ref docs.` は
`subject-full-stop` で失敗した。Footer 幅等は仕様への明示的追加制約であり、Type の大小文字は
仕様の非区別要件との不一致でもある。実際の発生頻度はこれらの例だけでは分からない。

### Angular

Angular のガイドは `<type>(<scope>): <short summary>` を使い、許可 Type、任意 Scope、
小文字の Subject、末尾ピリオドなしを定める。`docs` 以外は本文必須であり、
revert は `revert:` で始める[^angular]。ガイド記載の `test: add missing unit tests` と、
ガイドに沿って組み立てた `revert: feat(core): add signals` の例は成功した。ただし成功は
Angular の本文必須等を本ツールが検証したことを意味しない。

実際の履歴では Header 以外の制約にも衝突する。例えば
[Angular のコミット](https://github.com/angular/angular/commit/fee6dbf5264a30a80dfbe6b1babb7dc53215512d)
では本文中のコードブロック内 `TypeError: ...` が Trailer と認識され、`section-separator` error が出た。
この例は Angular ガイドがその件名を禁止するかではなく、自由形式の本文が Footer ヒューリスティックに
触れ得ることを示す[^format]。

### Drupal Core

Drupal Core の公式ガイドは 2025 年 11 月以降 Conventional Commits を採用し、
`chore` の代わりに `task` を使うと明記する。実在例
`task: #3565943 [backport] [security hardening] Update composer to 2.9.3` は
71 桁の warning に加え、既定の `type-enum` error で失敗した。同ガイド中の実コミット
`feat: #3582769 Add clickLink() to HttpKernelUiHelperTrait` と `By:` Footer は
warning のみで成功した[^drupal][^drupal-commit]。

`task` の例は、Conventional Commits の範囲内でプロジェクトごとに Type を選ぶ実例である。既定 Type の固定と設定変更可能性の両方を考慮する必要がある[^cc][^config]。

### Go と Linux kernel

Go 公式ガイドは変更対象の package を先頭に置き、
`math: improve Sin, Cos and Tan precision for very large arguments` を例示する。
この一行は `math` が既定 Type にないため `type-enum` error、65 桁の warning となった。
Go では `math` は変更種別でなく対象 package であり、文字列として偶然 Type に一致して通っても
同じ意味ではない[^go]。

Linux kernel の公式ガイドのパッチメール例にある `ext2: improve scalability of bitmap searching` は、
コミットの件名相当部分を入力すると `type-enum` error、`sub/sys: Condensed patch summary` は
`header-format` error となった。元のメール Subject はそれぞれ `[PATCH ...]` 付きであるが、
`git am` は通常 `[PATCH <anything>]` を取り除くので、メール Subject 全体をコミット Header と
混同しない[^kernel][^git-am]。Linux のガイドは summary を 70–75 文字、説明本文を 75 桁で
折り返すと述べており、本ツールの 72 表示桁 error より長い入力も許容し得る。
文字数と表示桁は一般に同一ではない。

### Django

Django は `Fixed Unicode bug in RSS API.`、チケットを閉じる
`Fixed #18307 -- Added git workflow guidelines.` など、過去形でピリオドを付ける流儀を定める。
前者は `header-format` error となった。バックポートには `[A.B.x]` 接頭辞も要求する[^django]。
これは Conventional Commits の Header とは異なる方針であり、`subject-full-stop` だけを
問題として捉えることはできない。

## 3. Git・GitHub の生成メッセージ

Git は `--fixup`、`--fixup=amend:` / `--fixup=reword:`、`--squash` でそれぞれ
`fixup! `、`amend! `、`squash! ` を作り、autosquash が扱う[^git-commit][^git-rebase]。
Git の revert や merge の典型形も含め、合成例 `fixup! feat: support changes`、
`Revert "feat: support changes"`、`Merge branch topic` は既定の ignore に一致し、
診断なしで成功した。本文を含む全ルールを回避するため、適合を検証した成功ではない。
`amend!` の置換メッセージは autosquash 後に残り得る[^config][^previous]。

GitHub の通常の PR merge は merge commit、squash merge はまとめた一つの commit を作る。
後者の既定件名は PR タイトル、コミット情報など、設定とコミット数に依存し、
編集もできる[^github-merge][^github-squash]。合成例 `Update documentation (#42)` は
`header-format` error となったが、これを全 GitHub squash merge の生成形とみなしてはならない。
GitHub の Revert 操作は元の merge commit を戻す新しい PR を作ると公式資料にあるが、
そこから固定の件名を推定していない[^github-revert]。

どの操作が `commit-msg` を実際に通るかは別問題である。既存調査は Git 生成件名、他の lint ツールの除外、フック起動の制約を詳しく扱う[^previous]。

## 4. 公開履歴の実測

同じ条件で二度実行し、以下の件数が一致した。2026-11-20 に
`gh api 'repos/<owner>/<repo>/commits?per_page=100'` で各 default branch の最新 100 件を取得し、
各 `commit.message` を上記 CLI に stdin で渡した。表のルール別列は、その error が
少なくとも一つ出た**コミット数**である。同一コミットに複数の error があれば複数列に数えられる。

| 公開リポジトリ | 失敗 / 100 | エラー別の失敗コミット数 |
| --- | ---: | --- |
| [angular/angular](https://github.com/angular/angular/commits/) | 55 | body-max-width 48、header-max-width 30、section-separator 7、footer-max-width 5、type-enum 2 |
| [conventional-changelog/commitlint](https://github.com/conventional-changelog/commitlint/commits/) | 86 | footer-max-width 75、header-max-width 11、body-max-width 4、header-format 1 |
| [conventional-changelog/conventional-changelog](https://github.com/conventional-changelog/conventional-changelog/commits/) | 68 | footer-max-width 36、header-max-width 31、body-max-width 18、trailer-format 1 |

取得した並びの先頭と末尾は、それぞれ Angular `ee18d5756387b6ba5a2ad2c10eee4f47fa3566f6` /
`a3bc17284656f259669abbc7ff28e04e4c69ca05`、commitlint `d360fa2021ec5ffa835f791329e00639fde35db8` /
`87714a2cc7fdcfcec3020d026159580d1ab36a93`、conventional-changelog
`0cdedf882b7d2c86ea8e03ce0c4fafc597fa2108` / `de5e1368096ea0805e51e20df587ce528ca0575b` である。
API の最新結果は変化するため、この両端だけでは全 100 件の固定された一覧にはならない。

- commitlint の `footer-max-width` は 75 件・75 行で、そのうち 74 行が `Co-authored-by:` で始まった。
  例えば
  [このコミット](https://github.com/conventional-changelog/commitlint/commit/ca77d367df1bbdf5d71c96fdde7191488b540247)
  の該当行は 79 桁である。これは Footer 全般の発生率ではなく、この標本内の検出行の内訳である。
- conventional-changelog の
  [このコミット](https://github.com/conventional-changelog/conventional-changelog/commit/43f19dfd521ec46e3a17ff3564b5aebca499d8ea)
  は本文先頭の `ConventionalGitClient#loadDeps() ...` が Trailer candidate と解釈され、
  `trailer-format` error となった。自由形式本文と Footer 判定が衝突する別の実例である[^format]。
- Angular の `type-enum` には、公式ガイドの Type 一覧にはない実際の
  `release: bump Angular DevTools version to 1.22.3` が含まれた[^angular-release]。
  実履歴が必ずしも記述された人間向けガイドと一致するとは限らない。

各リポジトリは著名な個別事例だが、その規模や採用率を定量化した根拠は今回集めていない。
最新 100 件は無作為抽出ではなく、ボット、merge 方法、整形、特定時期の活動の影響を受ける。
保存済みメッセージへの再 lint は、元のローカルフック入力、使用設定、実際にフックが走ったか、
利用者が操作時に拒否されたかを示さない。履歴入力と CLI のデフォルトフィルタも結果に影響し得る。
別バージョン、設定、時期への外挿はできない。

## 5. 設計上の読み取りと限界

本プロジェクトは表示幅の測定と、推奨と必須の warning / error の分離を設計原則に置く[^design]。
CLI は入力フィルタを既定で適用するが、Git の最終結果との完全一致は保証しない[^filtering]。
Conventional Commits 本体より厳しい既定値には、一貫したログ表示や検査という
利点と、長い URL・帰属情報・自由形式本文などを拒否するトレードオフがある。
生成形の全体除外も操作性と引き換えに幅・本文などの診断を失う[^config]。
一方、Go・Linux・Django の流儀を全面的に受け入れることは、Type の意味を検査する対象範囲自体を変える。
今回の観測だけからいずれかの設計変更を結論づけない。

本調査は挙げた一次資料と限定された標本の比較である。流儀の世界的な普及率、全メッセージへの適合率、拒否の主観的な不当性、設定変更後の完全な互換性は測っていない。

## 出典

[^cc]: Conventional Commits 1.0.0、仕様・例・FAQ: <https://www.conventionalcommits.org/en/v1.0.0/>
[^angular]: Angular Commit Message Guidelines: <https://github.com/angular/angular/blob/main/contributing-docs/commit-message-guidelines.md>
[^angular-release]: <https://github.com/angular/angular/commit/73916540c1688c0463617ef1469bf63adb0a628f>
[^drupal]: Drupal Core, The format of the Git commit message: <https://www.drupal.org/docs/develop/git/git-for-drupal-project-maintainers/the-format-of-the-git-commit-message>
[^drupal-commit]: <https://git.drupalcode.org/project/drupal/-/commit/53a72dc5eff2092c0086d8bc39e910cc3762cdf3>
[^go]: Go Contribution Guide, Good commit messages: <https://go.dev/doc/contribute#commit_messages>
[^kernel]: Linux kernel, Submitting patches, canonical patch format: <https://docs.kernel.org/process/submitting-patches.html#the-canonical-patch-format>
[^django]: Django, Committing guidelines and backports: <https://docs.djangoproject.com/en/dev/internals/contributing/committing-code/#committing-guidelines>
[^git-am]: Git, `git am`, DISCUSSION: <https://git-scm.com/docs/git-am#_discussion>
[^git-commit]: Git, `git commit`, `--fixup` / `--squash`: <https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---fixupamendrewordltcommitgt>
[^git-rebase]: Git, `git rebase --autosquash`: <https://git-scm.com/docs/git-rebase#Documentation/git-rebase.txt---autosquash>
[^github-merge]: GitHub, Pull request merges: <https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/about-pull-request-merges>
[^github-squash]: GitHub, Configuring commit squashing: <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/configuring-commit-squashing-for-pull-requests>
[^github-revert]: GitHub, Reverting a pull request: <https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/reverting-a-pull-request>
[^format]: [本リポジトリ: Message format](../references/message-format.md)
[^config]: [本リポジトリ: Configuration](../references/configuration.md)
[^design]: [本リポジトリ: Design principles](../design-principles.md)
[^filtering]: [ADR-0002: CLI の既定の入力フィルタ](../adr/0002-default-message-filtering.md)
[^previous]: [Git 生成件名に対する commitlint・gitlint の扱い](2026-11-17-generated-subjects-commitlint-gitlint.md)、およびそこから参照する Git 入力調査。

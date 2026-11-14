# commitlint パーサー実装設計 調査報告

- 調査日: 2026-11-09
- 対象バージョン: `@commitlint/parse` 21.2.3 / `conventional-commits-parser` 7.1.4
  / `conventional-changelog-conventionalcommits` 9.x / `conventional-changelog-angular` 9.x
- 方法: 各リポジトリ(`conventional-changelog/commitlint`,
  `conventional-changelog/conventional-changelog`)の main ブランチのソースを読み、
  Node.js 22 上で実際にパースさせて挙動を確認した。

## 後日訂正 (2026-11-13)

Git のフック入力を調べた後続調査により、この報告の Git 互換性に
関する推奨を次のとおり補足・訂正する。commitlint 自身のパーサー挙動の
記述は訂正対象ではない。詳細は
[後続調査](2026-11-13-git-commit-message-input.md)の 3〜4、8.2、9〜10 章を参照。

- 7 章の「Git 互換性のために前処理を合わせるべき」という推奨は、
  入力経路と目的を限定する必要がある。`commit-msg` フックが受け取るのは
  cleanup 前のファイルだが、cleanup の方式はフックから確定できない。
  `-m` / `-F` の既定ではコメント行が最終メッセージにも残るため、
  コメント行やシザー行を常に除去する前処理は Git と同じ結果に
  ならない場合がある。前処理を導入するならフック用など明示した
  入力モードに限定し、標準入力や Web 入力へ暗黙に適用しない。
- シザー行の文字列・行頭条件が Git と一致するという確認は維持する。
  ただし Git は LF 終端の行にだけ一致し、CRLF の行は一致しない。
  commitlint の前処理は `\r?\n` で行分割して CR を取り除くため、
  CRLF 入力では Git より広く一致しうる。文字列の一致だけで完全な
  Git 互換とはみなさない。
- コメント行の除去とシザー位置での切り捨ては、Git の cleanup 方式や
  `-v` / `commit.verbose` によって結果が異なる別の処理である。
  単一の前処理として一律に適用するのではなく、Git cleanup の再現を
  目的とするなら各条件とフックから判別できない限界を扱う必要がある。

## 1. 要約

- commitlint 自身はパーサーを持たない。`@commitlint/parse` は `conventional-commits-parser` の
  `CommitParser` を呼び出す 40 行程度の薄いラッパーである。
- `CommitParser` は **行単位の手書き状態機械 + 正規表現** で構成される。文法定義(PEG/BNF 等)も
  トークナイザーもなく、AST や位置情報(行・列・オフセット)を一切返さない。
- ヘッダーの構造(type/scope/subject)は `headerPattern` という 1 本の正規表現と、
  そのキャプチャグループ番号を名前に対応づける `headerCorrespondence` 配列で決まる。
  正規表現はプリセット(パーサープリセット)から注入されるため、
  パーサー本体は Conventional Commits を知らない汎用部品になっている。
- body と footer の境界判定は「footer トークンらしい行が現れたら以降はすべて footer」という
  一方向の状態遷移である。仕様(Conventional Commits 1.0.0)の footer 定義より緩く、
  本文中の `Note: ...` のような行で footer に切り替わる。
- パース結果に情報が足りないため、一部のルール(`body-leading-blank`, `footer-leading-blank`)は
  `parsed.raw` を再分割して行を探し直している。パーサーとルールの責務分担はきれいではない。

## 2. パッケージ構成と呼び出し経路

```
commitlint CLI
  └ @commitlint/load      設定と parserPreset を解決し parserOpts を作る
  └ @commitlint/lint      lint(message, rules, { parserOpts, parser, ... })
       ├ @commitlint/is-ignored   マージ・リバート・fixup! 等を正規表現で先に除外
       ├ @commitlint/parse        ← パーサーのラッパー
       │    └ conventional-commits-parser (CommitParser)
       │    └ conventional-changelog-angular (既定オプション)
       └ @commitlint/rules        各ルール関数 (parsed, when, value) => [valid, message]
```

`@commitlint/config-conventional` は `parserPreset: "conventional-changelog-conventionalcommits"` を
指定しており、実利用ではこのプリセットのオプションが angular の既定を上書きする。

### 2.1 `@commitlint/parse` の処理

1. `conventional-changelog-angular` を読み込み、その `parser`(旧 `parserOpts`)を既定値にする。
2. `fieldPattern: null` を強制する(後述のメタフィールド機能を無効化)。
3. ユーザー指定の parserOpts を上に重ねる
   (フラット形式・`parser` キー入れ子形式の両方を受け付ける)。
4. `new CommitParser(opts).parse(message)` を実行する。
   パーサー関数自体も差し替え可能(`opts.parser`)。
5. ヘッダーにマッチしなかった場合 `type`/`scope`/`subject` は `undefined` になるため、
   `null` に正規化する。
6. `parsed.raw = message` を付与する。

つまり commitlint 固有の解析ロジックはほぼなく、正規化と `raw` 付与だけである。

## 3. `CommitParser` の設計

### 3.1 構成要素

- `CommitParser.ts`(466 行): 状態機械本体。`lines` 配列と `lineIndex` カーソルを持つ。
- `regex.ts`: オプションから派生正規表現(notes / references / footerToken 等)を生成。
- `options.ts`: 既定オプション。
- `utils.ts`: 改行トリム、コメント行除去、シザー行での切り捨て、GPG 行除去、
  キャプチャ→フィールド割り当て。
- `stream.ts`: 複数コミットを async iterable / Node Stream で処理するラッパー。

コンストラクタで正規表現を一度だけ生成し、`parse()` ごとに状態(`lines`, `lineIndex`, `commit`)を
リセットする。インスタンスは使い回せるがスレッドセーフではない(JS なので問題にならない)。

### 3.2 出力モデル

```ts
interface CommitBase {
  merge: string | null
  revert: Record<string, string | null> | null
  header: string | null
  body: string | null
  footer: string | null
  notes: { title: string; text: string }[]      // BREAKING CHANGE 等
  mentions: string[]                              // @user
  references: { raw, action, owner, repository, prefix, issue }[]
}
type Commit = CommitBase & Record<string, string | null>  // type/scope/subject などは動的キー
```

特徴:

- `type`/`scope`/`subject` は型上の固定フィールドではなく、`headerCorrespondence` で決まる
  **動的キー** である。パーサーは「Conventional Commits の type」という概念を持たない。
- body/footer は **生文字列** であり、footer は個々のトレーラー(token/value)に分解されない。
  分解されるのは notes と references だけである。
- 位置情報はどこにも保持されない。

### 3.3 前処理

`parse(input)` の冒頭で以下を行う。

1. 空白のみの入力は `TypeError('Expected a raw commit')`。
2. 先頭・末尾の改行を除去し、`/\r?\n/` で行分割する。
3. `commentChar` 指定時は `# ------------------------ >8 ------------------------`(シザー行)
   以降を切り捨て、コメント行を除去する。
4. `gpg:` で始まる行を除去する。

**行の除去は元の行番号を失う** 操作であり、以後のどの段階でも元テキスト上の位置に戻れない。

### 3.4 状態遷移

```
parseMerge()          mergePattern にマッチすれば merge 行として 1 行消費
parseHeader()         次の 1 行を header とし、breakingHeaderPattern → headerPattern の順に照合
references ← header   ヘッダー中の issue 参照を抽出

isBody = true
while 行が残る:
    parseMeta()                     fieldPattern (-hash- 等) 形式のメタ欄。commitlint では無効
    if parseNotes(): isBody = false notes 正規表現 (BREAKING CHANGE: ...) にマッチ
    if !parseBodyAndFooter(isBody): isBody = false
                                    footerToken にマッチする行が来たら footer へ

parseBreakingHeader()   notes が空で header が `!` 付きなら subject を BREAKING CHANGE note にする
parseMentions(input)    元入力全体から @mention を抽出
parseRevert(input)      元入力全体に revertPattern を当てる
cleanupCommit()         body/footer/notes の前後改行除去、references の重複排除
```

重要な点:

- **ヘッダーは常に「最初の非除去行」** である。ヘッダーと本文の間の空行は要求されず、
  パーサーはそれを検査しない。`fix: c\nbody immediately` も正常にパースされ、
  `body = "body immediately"` となる。
  空行の有無は `body-leading-blank` ルールが `raw` を見て判定する。
- **body → footer は一方向** である。`isBody` は一度 `false` になると戻らない。
- `parseNotes` は再帰的で、note の継続行を `note.text` に追記し、
  別のトークン行(例 `Reviewed-by:`)に当たると note を閉じる
  (7.0.0 の "keep footers out of breaking notes" 修正)。
- mentions と revert は **行構造を無視して入力全体** に正規表現を当てる。
  前処理で除去したコメント行の中の `@foo` も拾いうる。

### 3.5 正規表現の内訳

conventionalcommits プリセットの値:

| オプション | 値 |
|---|---|
| `headerPattern` | `/^(\w*)(?:\((.*)\))?!?: (.*)$/` |
| `breakingHeaderPattern` | `/^(\w*)(?:\((.*)\))?!: (.*)$/` |
| `headerCorrespondence` | `['type', 'scope', 'subject']` |
| `noteKeywords` | `['BREAKING CHANGE', 'BREAKING-CHANGE']` |
| `revertPattern` | `/^(?:Revert\|revert:)\s"?([\s\S]+?)"?\s*This reverts commit (\w*)\./i` |
| `issuePrefixes` | `['#']` |

パーサー側で派生生成されるもの:

- **footerToken**: `^(?:BREAKING CHANGE|[\w-]+)(?::\s+|\s+(?:#)).+`(大文字小文字無視)。
  `Token: value` と `Token #123` の 2 形式。行頭固定で、インデントされた行は footer とみなさない
  (7.1.1 で修正)。
- **notes**: `^(?:\*\s+)?(BREAKING CHANGE|BREAKING-CHANGE):\s*(.*)`(大文字小文字無視)。
- **references**: `referenceActions`(close/fixes/resolved 等)を区切りに文を分割し、
  各文から `referenceParts` で `owner/repo#123` 形式を逐次抽出する。
  7.1.4 で sticky フラグ (`y`) を使った線形時間走査に書き換えられた。
  URL を含む文は参照抽出をスキップする。

`headerPattern` の scope 部が `(.*)` と貪欲であるため、`feat(a(b)): x` の scope は `a(b)` になる。

## 4. 実測で確認した挙動

conventionalcommits プリセット + `commentChar: '#'` で確認した。

- `feat(api)!: drop v1`: type=feat, scope=api, notes=[{BREAKING CHANGE, "drop v1"}]。
  `!` は notes に subject を複製する形で表現される。`breaking` フラグはない。
- `Update readme`: header のみ。type/scope/subject は `undefined`。
  ヘッダー不一致はエラーにならない。commitlint が `null` 化し、`type-empty` 等のルールで落とす。
- `fix: x\n\nThis explains.\nNote: something here\nmore body text\n\nRefs: #12`:
  body=`This explains.`, footer=`Note: something here\nmore body text\n\nRefs: #12`。
  本文中の `Note:` で footer に遷移し、以降の本文も footer 扱いになる。
- `feat: y\n\nbody\n\nBREAKING CHANGE: old api removed\ncontinues here\nReviewed-by: Z`:
  note.text=`old api removed\ncontinues here`。
  note の継続行は取り込み、次のトレーラーで閉じる。
- `fix: z\n\n  Key: value inside body`: body 扱い。インデント行は footer にならない。
- `fix: a\n\nCloses #3, #4 and owner/repo#5\nsee https://x.io/#7`:
  body=`""`(null でない), refs 3 件。`Closes #3` が footerToken にマッチし footer 化。
  body が空文字になる。`raw` に `", #4"` のような前置テキストが混入する。
  URL 内の `#7` は無視。
- コメント行・シザー行を含む入力: 除去済み。前処理で行ごと消える。
- `feat(ui): 日本語の件名`: 正常。ヘッダー正規表現は subject の文字種を問わない。

## 5. ルール側との責務分担

パース結果だけでは足りない検査を、ルールが `raw` から再計算している。

- `body-leading-blank`: `toLines(parsed.raw)[1] === ""` を判定。ヘッダー直後の行を見るだけなので、
  コメント行除去前の `raw` を前提にしている。
- `footer-leading-blank`: `rawLines.indexOf(footerLines[0])` で footer 開始行を探す。
  **footer 先頭行と同一文字列の行が本文中に先に現れると誤検出する** 構造である。
- `header-max-length` / `body-max-line-length` 等: JS の `String.prototype.length`
  (UTF-16 コードユニット数)で測る。全角文字も 1、サロゲートペアは 2 と数える。
  `*-max-line-length` は URL を含む行を無条件に免除する。
- `header-trim`: header 文字列の前後空白を見る。パーサーは header をトリムしない。
- `header-breaking-change`: header が `BREAKING CHANGE:` で始まるかを独自正規表現で判定。

また `@commitlint/is-ignored` がパース前にマージ・リバート・`fixup!`・semver タグ風メッセージ等を
正規表現で除外するため、パーサーの `mergePattern` は commitlint の経路では実質使われない。

## 6. 設計上の評価

### 利点

- **設定駆動の汎用性**: ヘッダー形式を正規表現とキャプチャ対応表で差し替えられるため、Angular 形式、
  Conventional Commits 形式、独自形式(例: `[JIRA-123] fix: ...`)に同一実装で対応できる。
  名前付きキャプチャにも対応する(6.2.0〜)。
- **寛容さ**: どんな入力でも例外を出さず何らかのオブジェクトを返す。
  changelog 生成(過去の汚れた履歴を読む用途)には適する。
- **実装の小ささ**: 本体 466 行で、読めば挙動を完全に把握できる。

### 欠点

- **位置情報がない**: 行除去と文字列連結で元の位置が失われる。
  エディタ連携(診断範囲の表示、クイックフィックス)には使えない。
  commitlint の CLI 出力が行番号を示さないのはこのためである。
- **仕様との乖離**: Conventional Commits 1.0.0 の footer は
  「本文の後の空行に続く」トレーラー群だが、パーサーは空行を区切りとして扱わず、
  行頭の `Word: ` だけで footer に遷移する。
  本文に `Note:` `Example:` のような行があると構造が崩れる。
  逆に、ヘッダー直後の空行欠落も構造上は許容される。
- **footer の構造化不足**: トレーラーが token/value に分解されないため、
  `trailer-exists` や `signed-off-by` といったルールは footer 文字列を自前で走査している。
- **動的キーの型安全性の低さ**: type/scope/subject が `Record<string, string | null>` に埋もれ、
  プリセット次第で存在しない。
- **changelog 用途との兼用**: references・mentions・revert・merge など
  lint に不要な抽出を毎回行う一方、lint に必要な情報(空行の位置、各行の位置)は持たない。

## 7. Rust での再実装に向けた示唆

commit-msg-lint(Rust、EAW 行長、CLI/LSP 両対応)の観点で、
commitlint のパーサーから引き継ぐべき点と変えるべき点を整理する。

**引き継ぐ価値がある点**

- 前処理(コメント文字、シザー行、改行コード)の仕様。`core.commentChar` に追従する点や、
  シザー行の厳密な文字列は Git 互換性のために合わせるべきである。
- ヘッダー不一致でも失敗せず「ヘッダー行」として返し、診断はルール層で出す方針。
  LSP では入力途中の不完全なメッセージを常に扱うため、寛容なパースは必須である。
- footer トークンを行頭固定・インデント除外とする判定(7.1.x の一連の修正は実際のバグ報告に基づく)。

**変えるべき点**

- **ロスレスな構文木を作る**: 除去せずにコメント行・シザー以降も「トリビア」としてノードに残し、
  全ノードにバイトオフセット(または行・列)を持たせる。LSP の診断範囲と CLI のエラー位置の両方が
  ここから導ける。
- **空行を構文上の区切りとして扱う**: header / 空行 / body 段落列 / 空行 / トレーラーブロック、
  という段落ベースの構造にし、「最後の段落がすべてトレーラー形式ならそれを footer とみなす」
  (`git interpret-trailers` に近い判定)にすると、本文中の `Note:` 問題を避けられる。
  空行欠落はパース失敗ではなく構文ノードの欠落として表現し、ルールが診断する。
- **ヘッダーを正規表現ではなく字句解析する**: type / `(` / scope / `)` / `!` / `:` / 空白 / subject
  を個別トークンとして位置付きで切り出すと、「`:` の後の空白がない」「scope の括弧が閉じていない」
  といった部分的な誤りに対して具体的な範囲で診断できる。カスタム形式の対応が必要なら、
  正規表現による差し替えは後から別経路で足せばよい。
- **トレーラーを token/value に分解する**: `BREAKING CHANGE` / `BREAKING-CHANGE` を特別扱いしつつ、
  継続行(インデント行)の規則を明示的に実装する。
- **長さ計測をルール層の関数として分離する**: commitlint は UTF-16 長で固定されているが、
  EAW 幅(East Asian Width の W/F を 2、A を設定で 1 または 2)で数える関数を差し込めるようにし、
  URL 免除のような例外もルール設定として明示する。
- **lint に不要な抽出は省く**: references・mentions・revert は必要になってから別パスで行えば足りる。

## 8. 参照したソース

- `commitlint/@commitlint/parse/src/index.ts`
- `commitlint/@commitlint/lint/src/lint.ts`
- `commitlint/@commitlint/rules/src/` 内の以下のファイル:
  `body-leading-blank.ts`, `footer-leading-blank.ts`, `header-max-length.ts`, `header-trim.ts`,
  `header-breaking-change.ts`, `body-max-line-length.ts`, `footer-max-line-length.ts`
- `commitlint/@commitlint/ensure/src/{max-length,max-line-length}.ts`
- `commitlint/@commitlint/is-ignored/src/defaults.ts`
- `commitlint/@commitlint/config-conventional/src/index.ts`
- `conventional-changelog/packages/conventional-commits-parser/src/` 内の以下のファイル:
  `CommitParser.ts`, `regex.ts`, `utils.ts`, `options.ts`, `types.ts`, `stream.ts`
- `conventional-changelog/packages/conventional-commits-parser/CHANGELOG.md`
- `conventional-changelog/packages/conventional-changelog-conventionalcommits/src/` 内の
  `parser.js`, `constants.js`
- `conventional-changelog/packages/conventional-changelog-angular/src/parser.js`

リポジトリ:
- [conventional-changelog/commitlint](https://github.com/conventional-changelog/commitlint)
- [conventional-changelog/conventional-
  changelog](https://github.com/conventional-changelog/conventional-changelog)

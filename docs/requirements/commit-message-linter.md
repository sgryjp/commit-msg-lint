# Rust 製コミットメッセージ Linter 要件

## 目的

`commit-msg-lint` は、チームがリポジトリ単位で共有するコミットメッセージ規約への適合を
検査する Rust 製 CLI である。既定では Conventional Commits を検査し、チーム固有の規約は
TOML 設定で変更する。

既存の commitlint または gitlint との CLI、設定、プラグイン API の互換性は提供しない。

## 範囲

初期リリースは次を提供する。

- コミットメッセージファイルまたは標準入力の検査
- Conventional Commits の構文解析と内蔵 Lint Rule
- `commit-msg-lint.toml` による内蔵 Rule の部分上書き
- 人間向けテキスト診断
- `--strict` による warning の失敗扱い

初期リリースは次を提供しない。

- Git 履歴中の単一コミットまたは範囲の検査
- Git フックのインストール、削除、変更
- JSON などの機械可読な診断
- プラグイン、スクリプト、外部バイナリによるカスタム Rule
- Footer の値の意味的な検査、`!` と `BREAKING CHANGE` の対応検査

Git フックからは利用者の既存のフック管理機構で次を呼び出す。

```sh
commit-msg-lint --edit "$1"
```

## CLI

| 呼び出し | 動作 |
| --- | --- |
| `commit-msg-lint --edit <path>` | `<path>` の UTF-8 テキストを検査する。 |
| `commit-msg-lint` | 標準入力の UTF-8 テキストを検査する。 |
| `commit-msg-lint --strict` | 標準入力を検査し、warning も失敗扱いにする。 |
| `commit-msg-lint --config <path>` | 指定した設定ファイルを使用する。入力指定と組み合わせられる。 |

`--edit` と標準入力以外の入力指定を同時に使うことはできない。未知のオプション、
必須値の欠落、矛盾する入力指定は CLI エラーである。

| 終了コード | 意味 |
| --- | --- |
| `0` | error がない。warning だけの場合も含む。 |
| `1` | 有効な Lint Rule の error 違反がある。`--strict` 時は warning がある場合も含む。 |
| `2` | CLI、設定、ファイル読み込み、UTF-8 デコードのエラーがある。 |

`--strict` は診断の重大度を変更せず、終了コードだけを変更する。

## 設定の探索と形式

`--config` が無い場合、カレントディレクトリを含む Git リポジトリのルートにある
`commit-msg-lint.toml` だけを探索する。設定が無い場合、または Git リポジトリ外で実行した
場合は内蔵プロファイルを使用する。

設定ファイルは TOML とし、先頭レベルの `version = 1` を必須とする。内蔵プロファイルへの
部分上書きであり、未指定の値は既定値を維持する。設定を完全に置き換える方式ではない。

読み込み不能、不正な TOML、未知の Rule ID、未知の設定キー、値の型・範囲・相互関係の
不正は設定エラーとする。既定値へのフォールバックはしない。

```toml
version = 1

[conventional]
types = ["feat", "fix", "docs", "refactor", "test"]
scope-required = true
allowed-scopes = ["cli", "parser", "docs"]

[display-width]
ambiguous-width = 1

[rules.header-max-width]
thresholds = [
  { limit = 50, severity = "warning" },
  { limit = 72, severity = "error" },
]

[rules.subject-full-stop]
enabled = false
```

`[conventional].types` を指定した場合、既定の type 一覧を完全に置き換える。`type-enum` が
有効なとき、空の `types` は設定エラーである。

`scope-required = true` は `scope-required` を有効にする。`allowed-scopes` を指定すると
`scope-enum` を有効にする。`allowed-scopes = []` は scope を一切許可しない。
`scope-required = true` と空の `allowed-scopes` の併用は設定エラーである。各 Rule テーブルで
`enabled = false` を明示した場合は、その無効化を優先する。

通常の Rule は次の形式で設定する。

```toml
[rules.<rule-id>]
enabled = true
severity = "error" # "warning" も指定可能
```

段階的しきい値を持つ Rule は `thresholds` を使う。`limit` は厳密な昇順とし、重大度は
warning から error へ同じか強くなる順序だけを許可する。違反時は該当する中で最も高い
重大度だけを一件報告する。

## メッセージの正規化

- 入力は有効な UTF-8 でなければならない。不正な UTF-8 は終了コード 2 の入力エラーである。
- LF と CRLF は同じ改行として扱う。
- 最終行にある一つの終端改行は内容、行長、空行の判定から除外する。
- 改行以外の C0/C1 制御文字、および双方向制御文字 U+202A–U+202E、
  U+2066–U+2069 は `control-character` の error とする。

## Conventional Commit の構文

Header は次の形である。

```text
type(scope)!: subject
```

- scope と `!` は任意である。
- type は ASCII 英字で始まり、その後に ASCII 英数字または `-` を続ける。
- scope は空でなく、Unicode 空白、`(`、`)`、改行を含まない任意の Unicode 文字列である。
- `: ` はコロン一つと空白一つでなければならない。`feat:subject` と `feat:  subject` は不正である。
- subject は空でない。subject 内の連続空白は許可する。
- `feat!:` と `feat(scope)!:` は受理する。

Body は Header を補足する任意の自由形式部分である。Footer は末尾に連続して置かれる
任意の Trailer 群である。Body または Footer がある場合、Header の直後には長さ 0 の空行を
ちょうど一行置く。Body と Footer の間も同様である。Body 内の段落区切りは許可する。

Trailer は Conventional Commits v1.0.0 の文法で認識する。

- token と `: ` または ` #`、値からなる。
- token 内の空白は `-` で表す。
- `BREAKING CHANGE` は token の例外として許可する。
- `BREAKING-CHANGE` は `BREAKING CHANGE` と同義とする。
- 値は複数行にでき、次の有効な Trailer が現れた時点で終わる。
- Header または Body の直後の区切り空行に続く末尾行が Trailer token の候補で始まる
  場合、区切りや値が不正でも Footer 候補とする。この場合は `trailer-format` を報告する。

Git 固有の Trailer 設定、区切り、別名は扱わない。

Header を解析できない場合、type、scope、subject に依存する Rule は報告しない。Header 全体
だけから検査できる Rule は引き続き実行する。

## Display Column

行長はバイト数や書記素クラスタ数ではなく Display Column で測る。参照実装は
`unicode-width` crate の文字列幅アルゴリズムである。

- `ambiguous-width = 1` は通常幅を使う既定値である。
- `ambiguous-width = 2` は CJK 幅を使う。
- 使用する crate と Unicode データの版は各リリースで固定し、更新時はリリースノートと
  適合テストで明示する。

## 内蔵 Lint Rule

すべての Rule ID は小文字 kebab-case の安定した公開名である。

- **`header-format` — error:** Header の type、任意 scope、任意 `!`、厳密な `: ` を検査する。
  空入力もこの Rule の違反である。
- **`type-enum` — error:** type を許可一覧と ASCII 大小文字を区別せず照合する。
- **`type-case` — error:** type が ASCII 小文字であることを検査する。
- **`scope-required` — disabled:** scope が存在することを検査する。
  `scope-required = true` で有効になる。
- **`scope-enum` — disabled:** scope を `allowed-scopes` と Unicode 完全一致で照合する。
  `allowed-scopes` 指定で有効になる。
- **`subject-empty` — error:** 解析された Header の subject が空でないことを検査する。
- **`header-trim` — error:** Header の先頭・末尾に空白がないことを検査する。
- **`subject-full-stop` — error:** subject が `.` で終わらないことを検査する。
- **`header-max-width` — warning/error:** Header の Display Column 数を検査する。
  `>50` は warning、`>72` は error である。
- **`section-separator` — error:** Header、Body、Footer 間の長さ 0 の空行をちょうど一行
  要求する。
- **`body-max-width` — error:** Body の各行が 72 Display Columns 以下であることを検査する。
- **`footer-max-width` — error:** Footer の各行が 72 Display Columns 以下であることを検査する。
- **`trailer-format` — error:** Footer 内の Trailer が Conventional Commits の文法に従うことを
  検査する。
- **`line-trailing-whitespace` — error:** Body と Footer の各行に末尾空白がないことを
  検査する。
- **`control-character` — error:** 禁止された制御文字と双方向制御文字がないことを検査する。

既定の type は `build`、`chore`、`ci`、`docs`、`feat`、`fix`、`perf`、`refactor`、`revert`、
`style`、`test` である。scope は既定で任意かつ無制限である。

## 診断

診断は標準エラー出力に一行ずつ出力する。正常時には出力しない。

```text
<source>:<line>: <severity>[<rule-id>] <message>
```

- `--edit` では `<source>` は指定パス、標準入力では `<stdin>` とする。
- 行全体を対象とする初期 Rule は列番号を出力しない。
- 複数の診断は行番号昇順、同じ行では rule ID の辞書順で出力する。

## 受け入れ条件

1. `feat: add parser` は既定設定で終了コード 0 になる。
2. 51–72 Display Columns の Header は warning を一件出し、通常は終了コード 0、
   `--strict` では 1 になる。
3. 73 Display Columns 以上の Header は `header-max-width` の error で終了コード 1 になる。
4. `feat: ` は `subject-empty`、`feat:subject` は `header-format` の error になる。
5. `feat!: change API` と `feat(api)!: change API` は受理される。
6. `BREAKING CHANGE: reason` を含む末尾 Footer は認識されるが、`!` の有無との整合は検査しない。
7. `allowed-scopes = []` のとき、scope を含む Header は `scope-enum` に違反する。
8. 同じ入力を LF と CRLF で渡しても同じ診断になる。
9. 不正な UTF-8、設定不備、CLI 不備は終了コード 2 になる。
10. Footer 候補の `Refs:#123` は `trailer-format` の error になる。

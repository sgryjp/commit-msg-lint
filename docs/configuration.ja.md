# 設定リファレンス

[English](configuration.md) | [日本語](configuration.ja.md)

英語版が正式な原文です。

`commit-msg-lint` は JSONC を使って組み込み設定を上書きします。指定しない設定には組み込みの既定値が
残り、設定ファイルによってプロファイル全体が置き換わるわけではありません。CLI と Web デモは同じ
設定形式を使用します。Web デモで設定が空の場合は既定値を使用します。

## 設定ファイルの生成

Git リポジトリのルートで、組み込みの既定値から編集可能な設定を生成します。

```sh
commit-msg-lint --print-default-config > commit-msg-lint.jsonc
```

シェルのリダイレクトは既存の `commit-msg-lint.jsonc` を上書きします。出力は編集可能な整形済み
JSON で、コメントや末尾カンマはありません。最新の公開スキーマを指す `$schema` メタデータを
含みます。Web デモで生成される既定設定にも同じメタデータが含まれます。必要に応じて生成された
ファイルをカスタマイズしてください。

## エディター用スキーマと互換性

省略可能な `$schema` キーは設定をエディター用スキーマに関連付けます。値は null でない文字列で
なければなりません。CLI と Web デモはこのキーを無視し、URL を取得しません。このキーのない既存の
設定も引き続き有効です。生成される既定設定は次を参照します。

<https://sgryjp.github.io/commit-msg-lint/commit-msg-lint.schema.json>

この Draft-07 スキーマは特定の CLI リリースではなく `main` に追従するため、インストール済みの
CLI と一致しない場合があります。代わりにインストール済み CLI に一致するスキーマを生成するには、
次の単独コマンドを使用します。

```sh
commit-msg-lint --print-config-schema > commit-msg-lint.schema.json
```

シェルのリダイレクトは出力先を上書きします。公開スキーマの代わりに使うには、エディターで生成された
スキーマを設定に関連付けてください。CLI は 1.0.0 未満であり、その設定スキーマは変更されることが
あります。必須の `"version": 1` は現在の形式を示すものであり、仕様の安定性を保証しません。

スキーマには、JSONC の編集時に末尾カンマの警告を避けるため、VS Code の
`allowTrailingCommas: true` 注釈が含まれます。これは VS Code の拡張であり、標準の JSON Schema
検証キーワードではありません。他のツールは無視する場合があります。
スキーマ検証ツールは JSONC ではなく JSON を要求する場合があります。また、重複キーや、必要な順序に
並んでいないしきい値の上限・重大度など、すべての設定エラーをスキーマ検証で検出できるわけでは
ありません。JSONC の文法とすべての設定ルールについては、設定パーサーが正式な判定基準です。
スキーマの `default` 注釈は参考情報であり、既定値を適用しません。省略された設定にはパーサーが
組み込みの既定値を適用します。

## 設定ファイルの探索

ファイルを明示的に選ぶには `--config <path>` を使用します。パスが絶対パスでなければ、現在の作業
ディレクトリからの相対パスです。明示的なパスは自動探索の代わりに使用され、読み取り可能な UTF-8
ファイルを指していなければなりません。ファイルの拡張子にかかわらず、内容は JSONC として解析されます。

`--config` がない場合、CLI は Git を使って現在の作業ディレクトリを含むリポジトリのルートを見つけ、
そのルートにある `commit-msg-lint.jsonc` だけを探します。サブディレクトリ内のファイルは探索しません。
ファイルがない場合や現在のディレクトリが Git リポジトリの外にある場合は、組み込みの既定値を使用します。
Git の実行失敗を含むその他の探索失敗はエラーです。

JSONC では JSON に加えて `//` 行コメント、`/* ... */` ブロックコメント、オブジェクトと配列の末尾
カンマを使用できます。引用符のないキーや単一引用符の文字列などの JSON5 拡張はサポートしません。
Rule ID を含む不明なオブジェクトキーや重複キー、および明示的な `null` 値は拒否されます。既定値を
維持するには、代わりに省略可能なフィールドを省略してください。不正な JSONC、読み取れないファイル、
不正な値や組み合わせは設定エラーです。これらの場合、既定値にはフォールバックしません。
設定エラー時の終了コードは `2` です。

[CLI のコメントと scissors のフィルタリング制御](cli.ja.md#メッセージのフィルタリング)は、呼び出しごとに独立した <!-- rumdl-disable-line MD089 -->
環境変数であり、共有 JSONC 設定の項目ではありません。明示的な `--config` は設定の探索を省略しますが、コメント
プレフィックスを得るための別の Git 照会まで必ず省略するわけではありません。

## 例

```jsonc
{
  "$schema": "https://sgryjp.github.io/commit-msg-lint/commit-msg-lint.schema.json",
  "version": 1,
  "conventional": {
    "types": ["feat", "fix", "docs", "refactor", "test"],
    "scope-required": true,
    "allowed-scopes": ["cli", "parser", "docs"],
  },
  "display-width": {
    "ambiguous-width": 2,
  },
  "rules": {
    // Warn at 51 columns and error at 73 columns.
    "header-max-width": {
      "thresholds": [
        { "limit": 50, "severity": "warning" },
        { "limit": 72, "severity": "error" },
      ],
    },
    "subject-full-stop": {
      "enabled": false,
    },
  },
}
```

必須のキーは最上位の整数 `version` だけで、値は `1` でなければなりません。他のすべてのオブジェクトと
キーは省略可能です。最小の設定ファイルは次のとおりです。

```json
{ "version": 1 }
```

## 無視するメッセージ

省略可能な `ignore` オブジェクトは、**すべての** Lint Rule を適用せずに除外するメッセージを制御します。
既定では `merge`、`revert`、`autosquash` はいずれも `true`、`header-patterns` は空の配列です。
各グループは個別に無効化できます。組み込みパターンをすべて無効化しても、カスタムパターンは追加で
適用されます。

```jsonc
{
  "version": 1,
  "ignore": {
    "merge": false,
    "revert": false,
    "autosquash": false,
    "header-patterns": ["^Release: ", "社内専用"],
  },
}
```

組み込みパターンは大文字と小文字を区別し、ASCII の半角スペースそのものを使用し、最初の Header の
先頭にマッチします。Header の残りの部分がパターンの境界で終わる必要はありません。以下の一覧で
`.*` は 0 文字以上を意味します。

- `autosquash`: `^fixup! `、`^amend! `、`^squash! `。
- `merge`: `^Merge branch `、`^Merge tag `、`^Merge remote-tracking branch `、
  `^Merge pull request `、`^Merge .* into .*`、`^Merged .* (in|into) .*`、`^Merged PR .*: .*`、
  `^Automatic merge`、`^Auto-merged .* into .*`。
- `revert`: `^(Revert|Reapply|revert|reapply) `。

autosquash の対象は空でも、Conventional 形式でなくても、別のプレフィックスを含んでもかまいません。
区切りのスペースは必須です。`fixup!invalid` は無視されません。`revert: ...` は Conventional
Header であり、組み込みの revert パターンにはマッチしません。merge に似た Body 行があるだけでは
無視されません。

`header-patterns` は `regex-lite` 構文を使用する空でない文字列の配列です。各パターンは最初の
Header だけを検索し、部分文字列にもマッチします。アンカーには `^` と `$` を使用してください。
Unicode リテラルはサポートしますが、Unicode 文字クラスはサポートしません。`\w`、`\s`、`\d` は
ASCII の定義を使用します。先読み・後読みと後方参照はサポートしません。空または不正なパターンは、
他の ignore がマッチしていても設定エラーです。`.*` のような有効なパターンはすべての Header に
マッチし得ます。エディター用スキーマはすべての正規表現構文を検証できないため、設定の解析が
正式な判定基準です。

マッチングはテキストに基づき、生成元には基づきません。手書きのメッセージもマッチすることがあり、
Git やホスティングサービスのメッセージでもマッチしないことがあります。特に GitLab のカスタム
マージテンプレートはすべてをテキストだけで認識できないため、必要に応じてパターンを設定してください。
autosquash の対象や `amend!` の置換メッセージはいずれも検証されません。マッチすると Body、Footer、
幅、制御文字を含むメッセージ全体が黙って除外され、`--strict` でも警告やエラーは発生しません。
これは、生成メッセージを扱いやすくするため、幅と重大度の一律の強制を犠牲にするトレードオフです。
マッチしないメッセージには、設定されたすべての Rule と重大度が適用されます。CLI は標準入力と
`--edit` では通常のフィルタリング後にこの方針を適用します。履歴入力ではエディター用フィルタリングを
行わず、Git のメッセージ出力に適用します。ライブラリと Web デモはフィルタリングされていない入力に
適用します。

## Conventional Commit の設定

省略可能な `conventional` オブジェクトには次の項目があります。

- `types`: 既定の許可 Type 一覧を置き換える文字列の配列です。既定値は
  `["build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style", "test"]`
  です。`type-enum` Rule は ASCII の大文字と小文字を区別せずに Type を比較します。別の
  `type-case` Rule が小文字を強制します。`type-enum` が有効な間は空の配列は無効です。
- `scope-required`: 真偽値で、既定値は `false` です。`true` の場合、`scope-required` Rule を
  有効にします。
- `allowed-scopes`: 文字列の配列です。既定では未指定で、Scope に制限はありません。設定すると
  `scope-enum` Rule が有効になり、Scope を大文字と小文字を区別して完全一致で比較します。空の配列は
  いかなる Scope も許可しません。`"scope-required": true` は、Rule の上書きでいずれかの Rule を
  無効にしていても、空の `allowed-scopes` 配列と組み合わせることはできません。

Rule 単位の `enabled` 設定はこれらの設定後に適用されるため、`scope-required` と `allowed-scopes` に
よって暗黙に決まる Rule の状態を上書きできます。`scope-enum` を明示的に有効にするには、空の配列の
場合も含め、`allowed-scopes` を指定する必要があります。

## 表示幅の設定

省略可能な `display-width` オブジェクトには次のキーが一つあります。

- `ambiguous-width`: `1`（既定値）または `2`。表示幅が曖昧な文字について、それぞれ通常の幅計算
  または CJK 幅計算を選択します。

幅に関する Rule はバイト数や文字数ではなく Display Column を測定します。実装には Unicode 17.0.0
データを使用する `unicode-width` 0.2.2 を使っています。

## Rule の設定

組み込み Rule を上書きするには、`rules` 内に Rule ID を名前とするオブジェクトを指定します。

```jsonc
{
  "version": 1,
  "rules": {
    "type-case": { "enabled": true, "severity": "warning" },
  },
}
```

- `enabled` は真偽値です。無効な Rule は診断を出力しません。
- `severity` は `"warning"` または `"error"` です。通常、警告では終了コードは `0` のままで、
  エラーでは終了コードが `1` になります。`--strict` では警告も終了コード `1` になりますが、報告される
  重大度は変わりません。
- `thresholds` は `header-max-width`、`body-max-width`、`footer-max-width` のみでサポートされます。
  以下を参照してください。

指定しない Rule 設定には既定値が残ります。無効な Rule についても設定は検証されます。Header の
構成要素に関する Rule（`type-enum`、`type-case`、`scope-required`、`scope-enum`、`subject-empty`、
`subject-full-stop`）は、Header を解析できる場合にのみ実行されます。

### 幅のしきい値

幅に関する Rule の `thresholds` 配列は既定のしきい値を置き換えます。各要素には整数の `limit` と
`severity` が必要です。

```jsonc
{
  "version": 1,
  "rules": {
    "body-max-width": {
      "thresholds": [
        { "limit": 60, "severity": "warning" },
        { "limit": 80, "severity": "error" },
      ],
    },
  },
}
```

上限値は 0 以上で、厳密な昇順でなければなりません。配列を空にすることはできません。上限値が大きく
なるにつれて、重大度は同じか `warning` から `error` へ上がる必要があります。行がしきい値に違反する
のは、幅が上限値を超えた場合のみです。複数のしきい値を超えた場合、Rule は超過した最大のしきい値の
重大度で診断を一つ報告します。

`header-max-width` の重大度を変更するには `thresholds` を使用する必要があり、Rule 単位の `severity`
は受け付けません。`body-max-width` と `footer-max-width` は Rule 単位の `severity` も受け付け、
既定のしきい値の重大度を変更します。一つの Rule オブジェクトに `severity` と `thresholds` の両方を
指定することはできません。その他の Rule は `thresholds` を受け付けません。

### 組み込み Rule

以下に記載した例外を除き、すべての Rule は既定で重大度 `error` で有効です。

| Rule ID | 既定値 | 検査内容 |
| --- | --- | --- |
| `header-format` | error | Header の構文: Type、省略可能な Scope と `!`、区切りの `: `。 |
| `type-enum` | error | Type が許可された `types` 一覧に含まれること。ASCII の大文字と小文字は区別しない。 |
| `type-case` | error | Type に ASCII の大文字が含まれないこと。 |
| `scope-required` | 無効 | Scope が存在すること。 |
| `scope-enum` | 無効 | 存在する Scope が `allowed-scopes` の要素に一致すること。 |
| `subject-empty` | error | Subject が空でないこと。 |
| `header-trim` | error | Header の先頭と末尾に空白がないこと。 |
| `subject-full-stop` | error | Subject が `.` で終わらないこと。 |
| `header-max-width` | 50 超で warning、72 超で error | Header の Display Column での幅。 |
| `section-separator` | error | Header、Body、Footer がちょうど 1 行の空行で区切られていること。 |
| `body-max-width` | 72 超で error | 各 Body 行の Display Column での幅。 |
| `footer-max-width` | 72 超で error | 各 Footer 行の Display Column での幅。 |
| `trailer-format` | error | Footer 候補が有効な Trailer で始まること。 |
| `line-trailing-whitespace` | error | Body と Footer の行末に空白がないこと。 |
| `control-character` | error | 行に禁止された制御文字や双方向文字が含まれないこと。 |

メッセージの構造と Rule の詳細な動作については[メッセージ形式リファレンス](message-format.ja.md)を、 <!-- rumdl-disable-line MD089 -->
CLI オプションと診断については [CLI リファレンス](cli.ja.md)を参照してください。 <!-- rumdl-disable-line MD089 -->

# Conventional Commits の Footer と Git trailer の実例調査

**調査日:** 2026-11-28

公式仕様、ツールの公式文書・ソース、プロジェクトのガイド、公開コミットを対象に
Conventional Commits の Footer と Git trailer の実例を調査した。
Web の調査には `ketch` を使い、以下プロジェクトをローカルにクローンして
refs・コミット・ガイドを確認した。

1. [angular/angular](https://github.com/angular/angular),
2. [conventional-changelog/commitlint](https://github.com/conventional-changelog/commitlint),
3. [python/cpython](https://github.com/python/cpython),
4. [git/git](https://github.com/git/git),
5. [torvalds/linux](https://github.com/torvalds/linux),
6. [mdn/translated-content](https://github.com/mdn/translated-content),
7. [rust-lang/rust](https://github.com/rust-lang/rust),
8. [textlint-ja/textlint-rule-preset-ja-technical-writing](https://github.com/textlint-ja/textlint-rule-preset-ja-technical-writing)

分類は **1. 公式に定義**（仕様や汎用ツール）、**2. 著名な慣例**（著名プロジェクトや複数の履歴に見られる）、
**3. その他**（個別方針・履歴のみの例・派生形）である。
「公式」は定義元の範囲内でのみ有効であり、第 2 層は世界的な採用率の順位ではない。
「実例」は保存されたコミットへのリンクであり、仕様中の見本や方針とは区別する。
また、実例はツールがその行を処理した証拠ではない。

## 要約

- Conventional Commits 1.0.0（以下 CC）が意味を規定する Footer は、破壊的変更を示す
  `BREAKING CHANGE` と同義の `BREAKING-CHANGE` である。`Reviewed-by`、`Refs`、`Acked-by` も仕様に登場するが、
  意味は規定されていない。Git も任意のキーを扱う。Git 本体の機能と Git プロジェクトの貢献方針は別である。
- `Fixes #123` は CC の構文に合い、GitHub などで issue を閉じる記法にもなるが、既定の Git では抽出されない。
  Linux の `Fixes: <SHA> ("<subject>")` は別の意味である。逆に `Refs:#123` は Git では抽出され得るが、
  CC の区切りではない。
- 8 クローンの非マージコミット計 34,256 件では、`Signed-off-by` と `Co-authored-by` が全クローンで見つかった。
  一方、`Fixes`・`Link` が多い Linux と、`Co-authored-by` を含むコミットが過半の commitlint では分布が異なる。
  キーの許可リストや幅制限を設計する場合は、構文・キーの意味・表示上の方針を分ける必要がある。

## 1. 公式に定義されるもの

### Conventional Commits 1.0.0

仕様 §8–10 は Footer の前に空行を置き、各 Footer を「単語トークン + `: ` または ` #` + 文字列値」とする。
トークン内の空白の代わりには `-` を使う。`BREAKING CHANGE` は空白を含む例外である。
値には空白と改行を含められ、次の有効なトークンと区切りに到達するまで続く。後続行のインデントは要件ではない。
仕様には Body なしで Header の後に Footer を置く例もある[^cc]。

| 形 | 仕様上の位置づけ | 実例 |
| --- | --- | --- |
| `BREAKING CHANGE: <description>` | §11–15 が破壊的変更を示すと規定する。大文字・コロン・空白・説明が必要で、破壊的変更は SemVer の MAJOR に対応する。Header の `!` でも示せる。`!` は Footer ではない。 | [Angular](https://github.com/angular/angular/commit/ad37f52c1212164c51ffcc533067af05c2c33c89)、[commitlint](https://github.com/conventional-changelog/commitlint/commit/ac2b3f4a947670e46af9a738881ba14158f0987a)、[textlint-ja](https://github.com/textlint-ja/textlint-rule-preset-ja-technical-writing/commit/608770201f9ab25a8f6baaf1bdfe1736a7eadd7c) |
| `BREAKING-CHANGE: <description>` | §16 が Footer トークンとして `BREAKING CHANGE` と同義とする。一般情報は大小文字を区別しないという §15 の例外が破壊的変更表記であり、小文字の表記を規範例にしない。 | Release Please の記載例[^release-please]。指定 8 クローンの `HEAD` から到達可能な全履歴を検索したが、行頭にこの大文字の形は見つからなかった。 |
| `Reviewed-by: Z`、`Refs: #123`、`Acked-by` | 前二つは仕様ページの Footer の例、最後は §9 のハイフンの例である。レビューの要件、参照先、チケットを閉じる効果は仕様が定めない。仕様は revert の説明でも `Refs: 676104e, a215868` を例示する。 | `Refs #4908` を共同著者行と別段落に置く [commitlint の例](https://github.com/conventional-changelog/commitlint/commit/3146d044186ccd2c4771823c01986d93c5144add)。`Refs:` ではなく ` #` 区切りである。 |

### Git の汎用構文と検出

Git は trailer をコミットオブジェクトの独立フィールドとして格納せず、メッセージから抽出する。
`git interpret-trailers` は ASCII 英数字と `-` からなるキーを認識し、先頭と内部に空白は置けない。
キーと区切りの間にはスペースやタブを置ける。メッセージ内の既定区切りは `:` で、直後の空白は必須ではない。
値はインデントした行に継続でき、`--parse` は `--only-trailers --only-input --unfold` に相当する。
`--trailer` 引数で受け付ける `=` は、既定のメッセージ内の区切りではない[^git-trailers]。

空行に続く末尾ブロックを対象とする。マニュアル上、全行が trailer のブロックか、Git 生成または設定済みの
trailer を少なくとも一つ含み trailer が 25% 以上のブロックを検出する。パッチ入力では既定で `---` 単独または
`--- ` で始まる行以降をメッセージ外とする。純粋なコミットメッセージなら `--no-divider` を選べる。
`trailer.separators` や `trailer.<alias>.key` で区切りや別名を設定できる。公式文書の
`trailer.ack.key=Acked-by` は `--trailer 'ack:…'` の展開例であり、`ack` が標準の別名ということではない。
`#` を区切りに設定しても、CC と同じ継続・空白規則にはならない[^git-trailers][^git-source]。

| 定義元・利用者 | キーや形 | 効果と根拠 | 実例 |
| --- | --- | --- | --- |
| Git | `Signed-off-by:` | `git commit -s` などがコミッターの名義で追加する。文面の意味はプロジェクトによる。Git と Linux の DCO は貢献を提出する権利等の表明であり、`git commit -S` の暗号署名でも、レビューの証明でもない[^git-commit][^dco][^git-project][^linux]。 | [Git](https://github.com/git/git/commit/acba8bf9753134d9bde89f94369f775838a8c5fe)、[Linux](https://github.com/torvalds/linux/commit/de59f08259545ba089fe7d33ac95f6b4b479abe0) |
| Git | `Message-ID:` | `git am --message-id` がパッチメールのヘッダーから付加する。メール上の `Message-ID:` 自体と保存された値の行は別である[^git-am]。 | [Linux](https://github.com/torvalds/linux/commit/2b66f2cc753243a6adb3e9a5c038b0fc59ba7ec9)（行の存在を確認。使用したコマンドまでは不明） |
| Git | `Cc:`、`*-by:` | `git send-email` はオプションに応じてパッチ本文の `Cc:` や `Signed-off-by:`、その他の `-by` 行から送信先を拾う。パッチ送信時の挙動であり、保存されたコミットがメールを送るわけではない[^git-email]。 | [Git の `Cc:`](https://github.com/git/git/commit/7454b68b6a08abdc87b8fea6ebc2c54f334860af) |
| GitHub | `Co-authored-by: Name <email>` | 複数の著者を帰属表示する。空行を挟んで末尾に置き、1 人につき 1 行を使う。アカウントへの帰属には当人のアカウントに関連付けられたメールアドレスを使う。Git オブジェクトの `author` ヘッダーを複数にする仕組みではない[^github-authors]。 | [commitlint](https://github.com/conventional-changelog/commitlint/commit/0022ebf9b483ffeaf2aa9e7c2df289523d31e766)、[MDN](https://github.com/mdn/translated-content/commit/6ed76d4901b002ce4d092d7f252a6b7b86389f28) |
| GitHub / GitLab | `Close(s/d)`、`Fix(es/ed)`、`Resolve(s/d)` + issue 参照 | 末尾に限らない終了キーワードである。GitHub ではコミットがデフォルトブランチに入ると閉じる。GitLab の既定パターンは `Closing`、`Fixing`、`Resolving`、`Implement(s/ed/ing)` も扱い、Self-Managed 側で変更できる。GitHub の `Fixes:` の例は PR 説明欄にも適用されるが、説明欄の記述はコミットとして残るとは限らない[^github-close][^gitlab-close]。 | [Angular の `Fixes #72027`](https://github.com/angular/angular/commit/1348dd1622f66c8f88c8ac0758cb0ee67a9febf6)、[MDN の `Fixes #39900`](https://github.com/mdn/translated-content/commit/01b903ca102678d89d61483744a42c714adf187c)（記載例であり、終了イベントの調査ではない） |
| Gerrit | `Change-Id: I…`、または所定の `Link:` | `commit-msg` フックによって追加される。末尾の `Change-Id` は、リポジトリ・ブランチと合わせて同じレビューの patch set を照合する。設定 `gerrit.reviewUrl` を使うフックは代わりに `Link: <review URL>` を挿入でき、対応するサーバーは同様に解釈する。任意の `Link:` が Gerrit ID の別名になるわけではない[^gerrit][^gerrit-hook]。 | [Chromium](https://chromium.googlesource.com/chromium/src/+/ad217e4ded08b22585fc9b2c7c3e4f9843d11ccf)、[Rust](https://github.com/rust-lang/rust/commit/7872d9239bfb126e9bcc2463db7bd1ce179149b0)、[Linux](https://github.com/torvalds/linux/commit/6a6db144d96352967623f38692dfc4b4fb814518)（後二者は行の観測であり、各プロジェクトの必須方針ではない） |
| Release Please | `Release-As: x.y.z` | 公式例では `main` のコミット本文から指定バージョンを読み、そのバージョン用のリリース PR を開く。大小文字を区別しない。`BEGIN_COMMIT_OVERRIDE` / `END_COMMIT_OVERRIDE` はマージ済み PR の本文で使う別機能である[^release-please]。 | [公式の作成例](https://github.com/googleapis/release-please#how-do-i-change-the-version-number) |
| git-buildpackage | `Gbp-Dch:`、`Thanks:`、`Closes:` / `LP:` | `gbp dch --meta` で Debian changelog を生成するためのタグ。`Gbp-Dch` の値は `Ignore`・`Short`・`Full`。旧名の `Git-Dch` も受け付ける。`Closes` と `LP` は既定のバグ終了タグだが、ここでの効果は changelog への記載である[^gbp]。 | 公式マニュアルに入力と出力の例がある[^gbp]。 |
| Copybara | `GitOrigin-RevId:` | コピー元のリビジョンを追跡するラベルの一つ。無効化でき、`CHANGE_REQUEST` ワークフローでは情報提供のみである[^copybara]。 | [projectqai/proto の実例](https://github.com/projectqai/proto/commit/e1ad01d70dca02c5ff5c505983b33acee336391c)。下記の `PiperOrigin-RevId:` と同一のキーではない。 |
| conventional-commits-parser / semantic-release | `noteKeywords`、`referenceActions`、`issuePrefixes` | パーサー既定のノートには `BREAKING CHANGE` と `BREAKING-CHANGE`、参照動詞には `close`・`fix`・`resolve` の各形がある。semantic-release は `parserOpts` で上書きでき、`BREAKING CHANGES` を追加する例がある。複数形は CC 1.0.0 の同義語ではない。参照抽出とホスティング側の issue 終了も別の処理である[^cc-parser][^semantic-release]。 | いずれも定義・設定例。設定が分からない履歴から解釈結果は判定できない。 |

Git には末尾に書かれても通常のキー値ではない形がある。`git cherry-pick -x` の `(cherry picked from commit …)` は
来歴のマーカーである。ブロック認識ではこの接頭辞を `Signed-off-by: ` と同じく Git 生成行として数えるが、
`%(trailers:only)` はキー値ではない行を返さない（[Linux の実例](https://github.com/torvalds/linux/commit/b865310c41f5b68c7de3bec86f205ba8cd284fd1)）。
`This reverts commit …` は revert の説明行である（[Rust の実例](https://github.com/rust-lang/rust/commit/37d2fa0d676aa55f80a7bfa8daace41c52bf65cc)）。
`git format-patch --base` の `base-commit:` と `prerequisite-patch-id:` はパッチ出力の情報である。
これらを保存されたコミットの汎用 trailer 一覧には含めない
[^git-cherry][^git-source][^git-patch][^angular]。

## 2. 著名なプロジェクト方針と複数にまたがる用例

| キー・形 | 運用上の意味 | 方針と実コミット |
| --- | --- | --- |
| `Reviewed-by:`、`Acked-by:`、`Tested-by:` | 大まかにはレビュー済み、了承、テスト済み。Linux は `Acked-by` が影響範囲の一部だけの場合や、技術的なレビューほど厳密でない場合を認める。Git プロジェクトでも十分に精査した当人が `Reviewed-by` を申し出る。 | Linux 方針[^linux]、Git プロジェクト方針[^git-project]。実例: Linux の [`Reviewed-by`](https://github.com/torvalds/linux/commit/0323a427052447e2c41c650fc98a6c04bf417e5d)、[`Acked-by`](https://github.com/torvalds/linux/commit/de59f08259545ba089fe7d33ac95f6b4b479abe0)、[`Tested-by`](https://github.com/torvalds/linux/commit/7ed616970b652ae76d99a01b468588e491021648)。 |
| `Reported-by:`、`Suggested-by:` | 報告者、変更を提案した人への帰属。Linux の `Reported-by` はバグ報告用で、原則として公開報告を指す `Closes:` が続く。一部のみを直す場合は `Link:` を使える。 | Linux 方針[^linux]、Git プロジェクト方針[^git-project]。実例: [`Reported-by` (Linux)](https://github.com/torvalds/linux/commit/686b8dbcedf8773dc505f9f315a0551c0bca70a3)、[`Suggested-by` と `Acked-by` (Git)](https://github.com/git/git/commit/1f8ddba9df4236540ac2ec35bf47f17f028f1f16)。 |
| `Co-developed-by:` | Linux の共同開発者。直後に同じ人の `Signed-off-by:` が必要で、GitHub の `Co-authored-by:` と置換できない。 | Linux 方針[^linux]、[Linux 実例](https://github.com/torvalds/linux/commit/39738c813c716c64a55767bfbe30da13f17e6dc3)、[Git にも存在](https://github.com/git/git/commit/26552cb62a5b375d4df651184941edf84f88a485)。 |
| `Fixes:` | Linux では不具合を導入したコミットの SHA-1 の先頭 12 桁以上と件名を参照する。stable 判定にも使われるが、`Cc: stable@vger.kernel.org` の代わりにはならない。FreeBSD もコミット ID と件名の形を定める。`Fixes: #123` や `Fixes #123` は同じ名前でも別の対象を指す。 | Linux 方針[^linux]、FreeBSD 方針[^freebsd]。実例: [`Fixes` と `Link` (Linux)](https://github.com/torvalds/linux/commit/d77700039d09479f81f0bc8b73002af8ec95d3f7)、[Angular の `Fixes: #69748`](https://github.com/angular/angular/commit/e23541b26385753ff2014d5f48cd1003f93cc27c)。 |
| `Closes:`、`Link:`、`Cc:` | Linux では公開バグ報告の URL、追加の参考資料・議論やパッチ投稿先、議論に含めた関係者や stable 宛て等をそれぞれ表す。`Closes:` の URL が任意のサービスで自動終了されるとは限らない。 | Linux 方針[^linux]。実例: [`Closes` (Linux)](https://github.com/torvalds/linux/commit/e509726660dd06ce107437aecf7c106c9d3fc1e7)、[`Cc` (Linux)](https://github.com/torvalds/linux/commit/de59f08259545ba089fe7d33ac95f6b4b479abe0)、[Git の `Closes`](https://github.com/git/git/commit/c0bec06cfedb930ecb3cd8c8cdb0f3e14e5e9308)。 |
| `Refs:` / `Refs #…`、`Ref:`、`References:` | 関連資料やチケットを示す例。似た綴りでも CC・Git が同義語として正規化する規定はない。終了キーワードとも異なる。 | CC の記載例[^cc]。実例: [commitlint の `Refs #4908`](https://github.com/conventional-changelog/commitlint/commit/3146d044186ccd2c4771823c01986d93c5144add)、[commitlint の `Ref:`](https://github.com/conventional-changelog/commitlint/commit/5383c9edcd9a351ea1c33ed49f47afed9b1cde6b)、[Rust の `References:`](https://github.com/rust-lang/rust/commit/8ce469e18b356dcbff768006d2bcebe9bd0a9e18)。 |
| `DEPRECATED:`、`Fixes #…`、`Closes #…` | Angular が Footer に記録する廃止予定と issue・PR 参照。`DEPRECATED:` は概要、空行、詳細・移行先を記す方針であり、破壊的変更とは異なる。 | Angular 方針[^angular]。実例: [`DEPRECATED:`](https://github.com/angular/angular/commit/8446e46f8bc33bd4419fa7f6106b8d117ca2e099)、[`Closes #60327`](https://github.com/angular/angular/commit/3a0f414727dc7a68e8fc1c7244a87640a4f21d08)。 |

方針と実履歴は一致するとは限らない。例えば Git プロジェクトのガイドは Linux の `Fixes:`・`Link:` を
Git で使う慣例ではないと説明するが、抽出結果には [`Fixes:` の実例](https://github.com/git/git/commit/c5fdd6e33a510b328c4a24dc2853d2c7c42a829a) がある。
`Co-authored-by` も GitHub の表示機能、Git プロジェクトの共同作業者への帰属、Drupal のハンドル名による帰属では、
求める値や意味が同一ではない[^github-authors][^git-project][^drupal]。

## 3. その他の方針・履歴だけから分かる形

| 定義元・観測先 | キー・形と用途 | 出典・実コミット |
| --- | --- | --- |
| Git プロジェクト | `Helped-by:`（実装パッチの形によらない手助け）、`Mentored-by:`（メンター）、`Based-on-patch-by:`（他者のパッチに基づくもの）。 | Git プロジェクト方針[^git-project]。キーの先頭の文字だけを大文字にすることも同プロジェクトの表記方針であり、Git パーサー一般の制約ではない。実例: [`Helped-by`](https://github.com/git/git/commit/1dab80e6839f6e4039e9ebd6e177dce7a48b657b)、[`Mentored-by`](https://github.com/git/git/commit/fec7b259babbb1d5ed4ac92b03bdb84872609814)、[`Based-on-patch-by`](https://github.com/git/git/commit/0efc4612a53757ab4c648bcb932ff1ac96b3cad4)。 |
| Drupal | `By:` に貢献者の Drupal.org ユーザー名。より細かく役割を示すキーも任意で利用可。 | Drupal 方針[^drupal]、[Drupal Core の `By:`](https://git.drupalcode.org/project/drupal/-/commit/53a72dc5eff2092c0086d8bc39e910cc3762cdf3)、[Pathologic のハンドル名による `Co-authored-by:`](https://git.drupalcode.org/project/pathologic/-/commit/49d3af921f6cc82dd5bae405c205c93868311e76)。 |
| OpenStack | Launchpad 用の `Closes-Bug:`（完了）、`Partial-Bug:`（部分修正）、`Related-Bug:`（関連）。 | OpenStack 方針[^openstack]。実例: [`Closes-Bug`](https://github.com/openstack/nova/commit/84a3e79fe1781c611a741c0808ab7366ad5f236a)、[`Partial-Bug`](https://github.com/openstack/nova/commit/f392bd017cbc140190eea1fb74f64027b4122292)、[`Related-Bug`](https://github.com/openstack/nova/commit/4b0167b8efd108b868c727214938a1a8134a768d)。Gerrit 共通のキーではない。 |
| Chromium | `Bug:`・`Fixed:`（issue の参照と解決）、`Test:`（試験内容）、`No-Presubmit:`・`No-Try:`・`No-Tree-Checks:`・`Cq-Include-Trybots:`（チェックや CQ の制御）、`Reviewed-on:`（レビュー URL）、`Cr-Commit-Position:`・`Cr-Branched-From:`（履歴位置・分岐）。 | CL footer 一覧[^chromium]。実例: [`Bug`、`Change-Id`、`Reviewed-on`、`Reviewed-by`、`Commit-Queue`、`Auto-Submit`、`Owners-Override`、`Cr-Commit-Position`](https://chromium.googlesource.com/chromium/src/+/ad217e4ded08b22585fc9b2c7c3e4f9843d11ccf)。その他の実例: [`Fixed`](https://github.com/chromium/chromium/commit/0eae3fb2baee633257794b5d62a6225e51b55ad4)、[`Test`](https://github.com/chromium/chromium/commit/2fe4fb47c92932a0d8fc41af18abc0e114cc47f1)、[`Cq-Include-Trybots`](https://github.com/chromium/chromium/commit/e8d84a7c749c3f701f51a51522b0c750f74fde92)、[`No-Presubmit`](https://github.com/chromium/chromium/commit/d3f46f535fffb688cee839cd45026a5c138b763c)、[`No-Try`](https://github.com/chromium/chromium/commit/d53574c2731a5aa655be33c42cff3bd46350d856)、[`No-Tree-Checks`](https://github.com/chromium/chromium/commit/a205c77f80b6dd0fcd639e6443969923a1325476)、[`Cr-Branched-From`](https://github.com/chromium/chromium/commit/b62c159c072cb6294811f09ca665b8e770c65dce)、[`Unresolved-Comment-Reason`](https://github.com/chromium/chromium/commit/94b780e39c1a7e21dbff4abdd4e14de765276a2d)。未掲載のキーの意味や作成者は履歴の行だけでは確定しない。 |
| FreeBSD | `PR:` は problem report、`Pull Request:` は PR/MR の URL。`Reviewed by:`、`Approved by:`、`Tested by:`、`Sponsored by:`、`Obtained from:` などの空白入りキーがある。`MFC after:`・`MFC to:`・`MFH:` は取り込み、`Relnotes: yes` はリリースノート候補、`Security:` はセキュリティ、`Differential Revision:` はレビュー URL。 | Committer's Guide[^freebsd]、[実コミットの `Reviewed by`、`Approved by`、`MFC after`、`Pull Request`](https://cgit.freebsd.org/src/commit/?id=4ba6fd2ef5a739b6fdc955890d27e07ae8794fb3)。空白入りキーは FreeBSD で正規でも、既定の Git trailer や CC Footer のキーとしては認識されない。 |
| LLVM の履歴 | `Differential Revision:` で Phabricator のレビューを参照する。 | [LLVM 実例](https://github.com/llvm/llvm-project/commit/f57f086714bc7a1399acf05d5ca1d665237cd725)。FreeBSD の上記方針とは別の観測例である。 |
| TensorFlow・Guava の履歴 | `PiperOrigin-RevId:` に数値の来歴 ID を記す。 | [TensorFlow 実例](https://github.com/tensorflow/tensorflow/commit/ec86ca456dd598e724aeca66427afee9a82fc505)、[Guava 実例](https://github.com/google/guava/commit/c9977bc9ce66082973a9114bba7ed88197a705a1)。 |
| Git・Linux・CPython・Rust・Angular の履歴 | `Assisted-by:` や `Claude-Session:` は支援ツール名・セッション URL を記す実例。採用方針や GitHub の帰属表示までは推定しない。 | [`Assisted-by` (Git)](https://github.com/git/git/commit/7d792a25622f6eadc7daa89c8144f2f75f20273b)、[Linux](https://github.com/torvalds/linux/commit/e057b8082759e9dfed7a77ee1cadcd37810681c1)、[CPython](https://github.com/python/cpython/commit/76d556e7a33d982f18a935f289e82d205696aadd)、[`Claude-Session` (Rust)](https://github.com/rust-lang/rust/commit/3cb7b3750325669981ed993dbc5f1c418a8e7f9a)、[Angular](https://github.com/angular/angular/commit/5d7303aeda280fe9dc215b8c2995f1e561b0ff32)。 |
| Git・CPython の履歴 | `Authored-by:`、`Originally-authored-by:`、`Original-patch-by:`、`Initial-work-by:` は元の著者や先行パッチへの帰属に使われる。`Noticed-by:`、`Diagnosed-by:`、`Inspired-by:` は気付き・原因の特定・着想への帰属例である。 | [`Authored-by` (CPython)](https://github.com/python/cpython/commit/f6d16a0d708c611dd96bb739f0896a0b6d7e04e8)、[`Originally-authored-by`](https://github.com/git/git/commit/20cd2c96a7b8930d451ec1189e80f7238f68d0ab)、[`Original-patch-by`](https://github.com/git/git/commit/a9456705f1884c95e597b25df9435b602caa07e5)、[`Initial-work-by`](https://github.com/git/git/commit/6cbc556c76b0f2d160115c564bb11b837637a9fb)、[`Noticed-by`](https://github.com/git/git/commit/84780db63657057bee11d898ad6c211730d4212f)、[`Diagnosed-by`](https://github.com/git/git/commit/ebb4d2ffa34e8c37796cc1be14216af5b91869aa)、[`Inspired-by`](https://github.com/git/git/commit/8c80498582bffc7b755406dbe3660e8b6098affe)。 |
| 履歴上の派生・綴り違い | `PR-Close:`、`Related:`、`Bug:`、`changelog:`、`Best-viewed-with:`、`Reported-and-tested-by:`、`Reviewed-multiple-times-by:`、`Reviwed-by:`。最後の形は `Reviewed-by` と似ているが、パーサーが自動修正する根拠はない。 | [Angular](https://github.com/angular/angular/commit/a5678f6f2b58470b86507cb78c2bd19bf19169a9)、[commitlint](https://github.com/conventional-changelog/commitlint/commit/5a6a4a833cdbde3d7589ed110f2e993b2bfa86b8)、[Git の `Bug:`](https://github.com/git/git/commit/10ab1af0f4446dcbc2b34773da9a9d0a9eb2b69f)、[Rust の `changelog: none`](https://github.com/rust-lang/rust/commit/b86a3a8e4c8702aa68af61e9eb621eb0fd46561f)、[Git の `Best-viewed-with:`](https://github.com/git/git/commit/8b9c5d2e3a38b6e0c2278fe10fe2a4bf34496a9d)、[Linux の複合形](https://github.com/torvalds/linux/commit/06cfbf21b33772266a03a580182b74233f41f2b2)、[CPython の複合形](https://github.com/python/cpython/commit/0c6422ff6a13ae309493fb7a358cb35d7ea959c8)、[Linux の綴り違い](https://github.com/torvalds/linux/commit/067db578cc2d16d40ba3b9908f8a0e2043203f3e)。 |

## 4. 8 クローンでの観測

各クローンのローカル `HEAD` を固定し、`refs/remotes/origin/HEAD` の指す先端と一致することを確認した。
集計には先端から到達する履歴だけを用いた。checkout の有無に依存しない `git log` と `git show <ref>:<path>` で
履歴とガイドを読んだ。Git 2.54.0 (Apple Git-157) で、各 `REV` に次のコマンドを実行した。

```sh
git -c trailer.separators=: log --no-merges -5000 \
  --format='%H%x09%(trailers:only,unfold,separator=%x1e)%x09%B%x00' "$REV"
```

抽出キーが `[a-z][\w-]{0,45}`（大小文字を区別しない）にマッチする行だけを数えた。
キーを小文字化し、同じコミット内の同じキーは 1 件とした。例は元のメッセージでも行の位置を確認した。
集計用の正規化であり、ツールや方針におけるキーの同義性を示すものではない。

| クローンと固定した `HEAD` | 件数 | `Signed-off-by` | `Co-authored-by` | その他の抽出例 |
| --- | ---: | ---: | ---: | --- |
| [Angular `63fa515ec7`](https://github.com/angular/angular/commit/63fa515ec74cb3690c2c48587d627c0c3120d0f0) | 5,000 | 1 | 29 | `Fixes` 28、`DEPRECATED` 4、`PR-Close` 1 |
| [commitlint `0022ebf9b4`](https://github.com/conventional-changelog/commitlint/commit/0022ebf9b483ffeaf2aa9e7c2df289523d31e766) | 3,639 | 53 | 2,058 | `Resolves`、`Ref`、`Related` 各 1 |
| [CPython `5dbe2ac850`](https://github.com/python/cpython/commit/5dbe2ac850b7b180685ed6008d5b9d459fd4934f) | 5,000 | 52 | 1,390 | `Assisted-by`、`Reviewed-multiple-times-by` 各 1 |
| [Git `acba8bf975`](https://github.com/git/git/commit/acba8bf9753134d9bde89f94369f775838a8c5fe) | 5,000 | 4,988 | 42 | `Helped-by` 272、`Reported-by` 164、`Acked-by` 161、`Suggested-by` 136、`Assisted-by` 131、`Mentored-by` 119、`Reviewed-by` 50、`Based-on-patch-by` 29 |
| [Linux `9437ee8b6c`](https://github.com/torvalds/linux/commit/9437ee8b6cf607e883b36e44bd755968daaebd48) | 5,000 | 4,945 | 4 | `Fixes` 3,432、`Link` 3,236、`Reviewed-by` 2,430、`Cc` 2,306、`Reported-by` 648、`Acked-by` 533、`Assisted-by` 458、`Closes` 417、`Tested-by` 392、`Suggested-by` 253、`Message-ID` 85、`Co-developed-by` 67 |
| [MDN translated-content `27ad40bf29`](https://github.com/mdn/translated-content/commit/27ad40bf29d2a34f92fb128ef38293da0f97b562) | 5,000 | 59 | 486 | `fr` などを除外（下記） |
| [Rust `063219d04d`](https://github.com/rust-lang/rust/commit/063219d04d8b3428506da3d8585c0221c07ca15b) | 5,000 | 207 | 72 | `Link` 6、`References` と `changelog` 各 3、`Change-Id` 1 |
| [textlint-ja `3a9833da89`](https://github.com/textlint-ja/textlint-rule-preset-ja-technical-writing/commit/3a9833da89624f90e37f39ac1bf0fea9d0072ecc) | 617 | 37 | 55 | `https` を除外（下記） |
| **合計** | **34,256** | **10,342** | **4,136** | 列は排他的ではない。 |

- **元の行との照合:** 件名を除いて大小文字を区別せずに検索すると、`^BREAKING CHANGE: ` は Angular 28 件、
  commitlint 14 件、Rust と textlint-ja 各 1 件、`^Fixes #` は Angular 385 件、commitlint 23 件、MDN 28 件であった。
  Linux の `(cherry picked from commit ` で始まる行は 203 件であった。いずれも出現コミット数で、Footer 率ではない。
  例えば CPython では本文中の [`Fixes #113329, which occurs …`](https://github.com/python/cpython/commit/025e7d278e29f3df2e95ed9daf9d9835760564c8) も拾う。
  逆に [Angular の `Fixes #72027`](https://github.com/angular/angular/commit/1348dd1622f66c8f88c8ac0758cb0ee67a9febf6) は末尾にある。
  `Refs #4908` のように複数の段落にメタ情報が散らばる例もある。
- **抽出と意図の差:** [MDN の `fr: auto-fix content issues`](https://github.com/mdn/translated-content/commit/cb487765976ec1fa11667c56e3e29c7dbb002cac) は、
  squash された件名のような末尾行を構文上 `fr` として抽出する例である。
  [Angular の URL](https://github.com/angular/angular/commit/3715109a2a3b17b88cfaf2129cb660ae600f6787) や
  [textlint-ja の URL](https://github.com/textlint-ja/textlint-rule-preset-ja-technical-writing/commit/40cae7a37274cecad968afd8f2dab66a429c2d3e) からは
  Apple Git 2.54.0 で `https` が抽出された。Git クローンの固定した `HEAD` は裸の `://` を除外する処理を持つため、
  これは全バージョン共通の挙動ではない[^git-source]。
  [変更コミット](https://github.com/git/git/commit/d122e37976050ba91a5ababdb54ee328380e9a9b) にも誤抽出の実例がある。
- **参照したクローン内文書:** Angular の Footer ガイド[^angular]、Git の貢献ガイド[^git-project]・実装[^git-source]、
  [Linux のパッチガイド](https://github.com/torvalds/linux/blob/9437ee8b6cf607e883b36e44bd755968daaebd48/Documentation/process/submitting-patches.rst)、
  [commitlint の Commit Rules](https://github.com/conventional-changelog/commitlint/blob/0022ebf9b483ffeaf2aa9e7c2df289523d31e766/.github/CONTRIBUTING.md)、
  [CPython の開発ガイドへの案内](https://github.com/python/cpython/blob/5dbe2ac850b7b180685ed6008d5b9d459fd4934f/.github/CONTRIBUTING.rst)、
  [MDN の寄稿ガイド](https://github.com/mdn/translated-content/blob/27ad40bf29d2a34f92fb128ef38293da0f97b562/CONTRIBUTING.md)、
  [Rust の寄稿ガイド](https://github.com/rust-lang/rust/blob/063219d04d8b3428506da3d8585c0221c07ca15b/CONTRIBUTING.md)、
  [textlint-ja の README](https://github.com/textlint-ja/textlint-rule-preset-ja-technical-writing/blob/3a9833da89624f90e37f39ac1bf0fea9d0072ecc/README.md)。
  ガイドに一覧のないキーは、履歴から採用方針を推定していない。

デフォルトブランチの非マージ履歴を先端から選んでおり、無作為抽出ではない。対象期間はプロジェクトごとに異なり、
squash、下流コードの取り込み、ボットによる反復コミットの影響もある。件数は Git の版と抽出条件に依存する。
Footer の総数、全表記の採用率、自動処理の成功率、開発者数を示すものではない。

## 5. 曖昧な行と本ツールへの含意

次表の入力は合成例である。`feat: change` と空行の後に示した行だけを置き、空の一時リポジトリで
`git -c trailer.separators=: interpret-trailers --parse` を実行した。
CC 欄は仕様との比較であり、特定の CC 実装の出力ではない[^cc][^git-trailers]。

| 行 | CC Footer の形 | Git 2.54.0 の出力 |
| --- | --- | --- |
| `BREAKING CHANGE: API v2` | 規範形 | なし |
| `BREAKING-CHANGE: API v2` | 規範上の同義語 | `BREAKING-CHANGE: API v2` |
| `Fixes #123` | 合う | なし |
| `Refs:#123`、`Refs : #123` | どちらも区切り不適合 | どちらも `Refs: #123` |
| `Reviewed by: A` | トークンに空白。FreeBSD での正規形 | なし |
| `Refs: ` | 有用な値なし。本ツールでは空値は無効 | 空値の `Refs:`（`--trim-empty` なし） |
| `Refs: first` の次行が `second` | 改行を値に含められる | なし。次行を ` second` とすると `Refs: first second` |
| `https://example.com/path` | Footer の区切りに合わない | `https: //example.com/path`（使用した版のみ） |

実メッセージでは境界が曖昧になる。[Angular の `Fixes https://…`](https://github.com/angular/angular/commit/69c87301aa545adb2c842cac31f062848c48ede9) は
末尾で issue を参照しているが、CC の区切りは使っていない。Angular ではこの形の行を含むコミットが 10 件あった。
逆に `TypeError: …` や `PR #…` で始まる行は、形が似ていても自由形式の本文であり得る。
`Reviwed-by:` や `Co-Authored-By:` は Git のキー構文に合うが、未知キーや誤記の扱い、大小文字による処理の違いは
受け側のツールやプロジェクトの方針による。Linux は `Fixes:` を折り返さないよう明記しており、URL・名前・アドレスも
そのままの形で機械処理に必要な場合がある[^linux][^github-authors]。

本ツールは末尾ブロックを抽出せず、本文を前から走査し、最初の有効な Trailer または特定位置の候補行から後を
Footer とみなす。その行が無効な候補なら `trailer-format` を報告するが、後続行の構文は一律に検証しない。
壊れた区切りを見つけやすい反面、前回調査の [Angular の `TypeError:`](https://github.com/angular/angular/commit/fee6dbf5264a30a80dfbe6b1babb7dc53215512d)
のように本文を Footer と誤認し得る。一方、既定の Git の抽出に従うだけでは、空白入りの `BREAKING CHANGE:` や
`Fixes #…` を拾えない[^format][^previous]。

本ツールの設計方針は、表示桁を計測し、推奨と必須を warning と error で分け、Git の入力整形を近似することである。
これらは構文やキーの意味とは別の判断である。長い Footer を知らせる利点はあるが、`Co-authored-by` や URL が
幅制限で拒否されると、機械可読な形を保ったまま通せないことがある。キーの許可リストは誤認を減らし得るが、
CC が許す独自キーまで拒否し得る。Git の抽出、CC の構文、サービスやプロジェクトの固有ルールを分ければ、
違反した規則を明確にできる反面、区分や設定が増える。今回の標本だけで既定ルールの変更は決めない
[^design][^format][^previous]。

## 出典

[^cc]: Conventional Commits 1.0.0（仕様 §8–16・Examples・FAQ）: <https://www.conventionalcommits.org/en/v1.0.0/>
[^git-trailers]: Git, `git interpret-trailers`（抽出、整形、設定、例）: <https://git-scm.com/docs/git-interpret-trailers>
[^git-source]: Git クローンで確認した `trailer.c`（`git_generated_prefixes`、`find_separator`、末尾ブロック判定）: <https://github.com/git/git/blob/acba8bf9753134d9bde89f94369f775838a8c5fe/trailer.c>
[^git-commit]: Git, `git commit`（`--signoff`、`--trailer`、`--gpg-sign`）: <https://git-scm.com/docs/git-commit>
[^git-am]: Git, `git am`（`--message-id`、`am.messageid`）: <https://git-scm.com/docs/git-am>
[^git-email]: Git, `git send-email`（`--signed-off-cc`、`--suppress-cc`）: <https://git-scm.com/docs/git-send-email>
[^git-cherry]: Git, `git cherry-pick -x`: <https://git-scm.com/docs/git-cherry-pick>
[^git-patch]: Git, `git format-patch`（Base Tree Information）: <https://git-scm.com/docs/git-format-patch#_base_tree_information>
[^dco]: Developer Certificate of Origin 1.1: <https://developercertificate.org/>
[^git-project]: Git プロジェクトの `Documentation/SubmittingPatches`、クローンの固定 ref: <https://github.com/git/git/blob/acba8bf9753134d9bde89f94369f775838a8c5fe/Documentation/SubmittingPatches#commit-trailers>
[^linux]: Linux kernel, Submitting patches: <https://docs.kernel.org/process/submitting-patches.html>。stable の詳細: <https://docs.kernel.org/process/stable-kernel-rules.html>
[^github-authors]: GitHub, Creating a commit with multiple authors: <https://docs.github.com/en/pull-requests/committing-changes-to-your-project/creating-and-editing-commits/creating-a-commit-with-multiple-authors>
[^github-close]: GitHub, Linking a pull request to an issue（コミットメッセージにも適用）: <https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue#linking-a-pull-request-to-an-issue-using-a-keyword>
[^gitlab-close]: GitLab, Manage issues（Closing issues automatically）: <https://docs.gitlab.com/user/project/issues/managing_issues/#closing-issues-automatically>
[^gerrit]: Gerrit, Change-Ids: <https://gerrit-review.googlesource.com/Documentation/user-changeid.html>
[^gerrit-hook]: Gerrit, `commit-msg` hook（`gerrit.reviewUrl`）: <https://gerrit-review.googlesource.com/Documentation/cmd-hook-commit-msg.html>
[^release-please]: Release Please, README（破壊的変更・バージョン指定・PR 本文による上書き）: <https://github.com/googleapis/release-please>
[^gbp]: Debian の `gbp-dch(1)`（META TAGS）: <https://manpages.debian.org/unstable/git-buildpackage/gbp-dch.1.en.html#META_TAGS>
[^copybara]: Copybara, リファレンス（`GitOrigin-RevId` などの来歴ラベル）: <https://github.com/google/copybara/blob/master/docs/reference.md>
[^cc-parser]: Conventional Changelog, `conventional-commits-parser` の既定オプション: <https://github.com/conventional-changelog/conventional-changelog/blob/master/packages/conventional-commits-parser/src/options.ts>
[^semantic-release]: semantic-release, commit-analyzer（`parserOpts` の例）: <https://github.com/semantic-release/commit-analyzer#usage>
[^angular]: Angular, Commit Message Guidelines、クローンの固定 ref: <https://github.com/angular/angular/blob/63fa515ec74cb3690c2c48587d627c0c3120d0f0/contributing-docs/commit-message-guidelines.md#commit-footer>
[^drupal]: Drupal Core, The format of the Git commit message: <https://www.drupal.org/docs/develop/git/git-for-drupal-project-maintainers/the-format-of-the-git-commit-message>
[^openstack]: OpenStack, Set Up and Learn GIT（Including External References）: <https://docs.openstack.org/contributors/common/git.html>
[^chromium]: Chromium, CL footer reference: <https://chromium.googlesource.com/chromium/src/+/HEAD/docs/contributing.md#cl-footer-reference>
[^freebsd]: FreeBSD, Committer's Guide（Commit Log Messages）: <https://docs.freebsd.org/en/articles/committers-guide/>
[^format]: [本リポジトリ: Message format](../references/message-format.md)
[^design]: [本リポジトリ: Design philosophy](../../README.md#design-philosophy)
[^previous]: [コミットメッセージ流儀と既定設定の適合性](2026-11-20-commit-message-conventions.md)

# Commit Message Linter

チームが共有するコミットメッセージ規約を検査するコンテキストである。

## Language

**Conventional Commit**:
Conventional Commits 仕様に従い、type、任意の scope、subject から構成されるコミットメッセージ。
_Avoid_: Conventional Commits 形式、規約付きコミット

**Type**:
コミットの変更種別を表す、Header の先頭に置かれる識別子。
_Avoid_: category、kind

**Scope**:
Type が表す変更の対象をより具体化する、任意の Header 構成要素。
_Avoid_: module、area

**Subject**:
Header で変更内容を短く表す、type と任意の scope に続く必須の要約。
_Avoid_: description、summary

**Header**:
コミットメッセージの先頭行であり、Conventional Commit の type、任意の scope、subject を含む部分。
_Avoid_: title、件名

**Body**:
Header を補足する、任意の自由形式の説明部分。
_Avoid_: description、message

**Footer**:
Header または Body の後に置かれる、任意の Trailer 群からなる構造化された補足情報の部分。
_Avoid_: metadata

**Trailer**:
Footer 内の token と値からなる構造化された補足情報の項目。
_Avoid_: footer entry

**Display Column**:
コミットメッセージの行長を測るための、Unicode 表示幅に基づく仮想的な一列分の幅。
_Avoid_: byte length、character count

**Lint Rule**:
コミットメッセージの規約適合を判定し、違反時に重大度を伴う診断を生成する規則。
_Avoid_: check、validator

**Commit Message Linter**:
チームのコミットメッセージ規約への適合を判定するツール。
_Avoid_: formatter、commit generator

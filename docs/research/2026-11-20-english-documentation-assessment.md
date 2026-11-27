# 英語ドキュメントの読みやすさ評価

調査日: 2026-11-20。対象はコミット `3bc3807e7d3b` 時点の利用者向け主要リファレンスと、一部の開発・設計文書である。欧州評
議会、Google、Microsoft の一次資料を基に、測定結果と評価上の限界を整理する。

## CEFR レベルの扱い

- CEFR の記述は、読解などの活動について、言語使用者が何をできるかを示す。例えば B2 の読解には、専門分野の技術的な議論を
  含む複雑な文章の主要な考えを理解できるという記述がある。文章単体の固定的な等級とは区別する。出典: [欧州評議会 CEFR 記
  述尺度](https://rm.coe.int/168045b15e)（global scale）。
- 欧州評議会の読解・聴解評価ガイドは、言語、話題・内容、文章と課題の認知的複雑さを区別し、個々の要因とレベルには直接の対
  応がなく、要因の相互作用が難しさを生むとする。長い文章が必ずしも難しいわけではなく、見出しなどの視覚的支援や、情報を探
  す課題の条件も関わる。ガイドの語数例は評価課題向けの概数であり、一般文書の CEFR 判定基準ではない。出典: [A CEFR
  Reference Guide for Assessment
  Tasks](https://rm.coe.int/a-cefr-reference-guide-for-assessment-tasks-reading-and-listening/1680a9178e)（Text features
  ）。
- したがって、文章に CEFR を当てる場合は、対象読者、専門知識、読む目的や求める理解を明示した暫定的な見立てにとどめる。
  単語や文の長さだけでは決められず、特定のツールが文章の CEFR レベルを精密に算出できるとは扱わない。
  これは上記資料からの評価上の帰結である。

## 評価の観点

- **平易さと文の長さ**: Google は平易な語、短い文、曖昧さのない表現を推奨する。別の技術文書教材では、一文を一つの考えに
  絞り、必要に応じて文やリストに分ける。Microsoft も簡潔な文と正確で平易な語を推奨する。ただし、明確さや親しみやすさを損
  なうほど削らない。ここで参照した両社のガイドは、一文の語数に普遍的な合否閾値を設けていない。出典: [Google: Write for a
  global audience](https://developers.google.com/style/translation)、[Google: Short
  sentences](https://developers.google.com/tech-writing/one/short-sentences)、[Microsoft: Use simple words, concise
  sentences](https://learn.microsoft.com/en-us/style-guide/word-choice/use-simple-words-concise-sentences)。
- **能動態**: 両社とも原則として能動態を使い、動作主体を明確にする。ただし受動態にも、対象を強調する、主体が不要な場合や
  読者を責める表現を避ける場合などの用途がある。受動態の検出だけで欠点と断定しない。出典: [Google: Active
  voice](https://developers.google.com/style/voice)、[Microsoft:
  Verbs](https://learn.microsoft.com/en-us/style-guide/grammar/verbs)。
- **専門用語と略語**: 不要なジャーゴンは避けつつ、技術的な事柄は正確に表す。対象読者に不慣れな用語は定義するか説明へのリ
  ンクを付け、同じ概念には同じ用語を使う。略語は読者の負担を考え、必要なものを初出時に説明する。出典: [Google:
  Words](https://developers.google.com/tech-writing/one/words)、[Google: Write for a global
  audience](https://developers.google.com/style/translation)、[Microsoft: Top 10
  tips](https://learn.microsoft.com/en-us/style-guide/top-10-tips-style-voice)、[Microsoft: Use simple words, concise
  sentences](https://learn.microsoft.com/en-us/style-guide/word-choice/use-simple-words-concise-sentences)。
- **見出しと構成**: 両社とも内容を具体的に示す見出しと sentence case を推奨する。Google は作業の見出しを動詞の原形、概念
  の見出しを名詞句とし、階層を飛ばさないよう求める。Microsoft は見出しを短く具体的にし、同じ階層では構文を並行させる。作
  業見出しには *To create ...* のような不定詞句も提案しており、両社の形式は完全には同じでない。出典: [Google: Headings
  and titles](https://developers.google.com/style/headings)、[Microsoft:
  Headings](https://learn.microsoft.com/en-us/style-guide/scannable-content/headings)。
- **条件と翻訳**: Google は条件や目的を指示の前に置くことを勧める。Microsoft は翻訳のため、短く単純な文、
  冠詞や関係詞の明示、慣用表現の回避を勧める。機械翻訳向けには文の断片も控えるよう求めている。
  出典: [Google: Sentence structure](https://developers.google.com/style/sentence-structure)、
  [Microsoft: Writing tips](https://learn.microsoft.com/en-us/style-guide/global-communications/writing-tips)。

## 測定結果

対象は導入判断と利用時に参照される 5 ファイル、開発向けの CONTRIBUTING、用語集、英語の ADR 1 件とした。日本語の ADR や設
計書、過去の調査記録などは対象外であり、全ドキュメントの網羅的な評価ではない。数値はテキスト抽出の方法に依存する概算であ
る。

| ファイル | 文数 | 語数 | 平均語数/文 | 25 語超の文 | FKGL | FRE |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| [README](../../README.md) | 40 | 584 | 14.6 | 2 (5%) | 10.3 | 47.1 |
| [CLI](../references/cli.md) | 71 | 1,014 | 14.3 | 4 (6%) | 10.1 | 47.6 |
| [Configuration](../references/configuration.md) | 105 | 1,249 | 11.9 | 1 (1%) | 10.0 | 44.6 |
| [Message format](../references/message-format.md) | 97 | 1,426 | 14.7 | 7 (7%) | 9.9 | 49.7 |
| [Agent usage](../agent-usage.md) | 22 | 317 | 14.4 | 1 (5%) | 9.6 | 51.8 |
| [Contributing](../../CONTRIBUTING.md) | 44 | 596 | 13.5 | 2 (5%) | 8.3 | 58.9 |
| [Glossary](../../GLOSSARY.md) | 14 | 225 | 16.1 | 0 (0%) | 10.6 | 48.0 |
| [Display-column ADR](../adr/0001-display-column-measurement.md) | 5 | 97 | 19.4 | 1 (20%) | 11.1 | 50.2 |

特に ADR は抽出された文が 5 文だけのため、数値を他の文書と単純に比較しない。

FKGL は Flesch–Kincaid Grade Level の機械的な推定値で、米国の学年尺度に基づく。FRE は Flesch Reading Ease で、一般に高
いほど平易とされる。どちらも語や文の長さに基づく参考値であり、専門知識、正確さ、情報密度や実際の理解度を測るものではな
い。両者を CEFR に換算しない。25 語超は確認箇所を探すための便宜的な閾値であり、Google や Microsoft の基準ではない。

### 抽出方法

`markdown-it-py` 4.3.0 の CommonMark と table 拡張で段落とリスト内の段落を抽出した。見出し、表、コードブロック、リンク先
URL は除き、インラインコードは一律 `item` に置き換えた。末尾が `.`, `!`, `?` の部分を文として数え、英字を基に語数を数え
た。FKGL と FRE はその抽出文を連結し、`textstat` 0.7.13 で算出した。文区切りや省略形は完全には解析できず、`textstat` 独
自の文数も表の文数と一致するとは限らない。コードや識別子を実際に読む負担は数値に反映されない。

再現用のコマンドは次のとおり。`/path/to/script.py` は、後に示すコードを保存したファイルのパスに置き換える。

```sh
uv run --with textstat==0.7.13 --with markdown-it-py==4.3.0 python /path/to/script.py \
  README.md docs/references/cli.md docs/references/configuration.md \
  docs/references/message-format.md docs/agent-usage.md \
  CONTRIBUTING.md GLOSSARY.md docs/adr/0001-display-column-measurement.md
```

スクリプトの内容は次のとおり。

```python
import re
import statistics
import sys

import textstat
from markdown_it import MarkdownIt

parser = MarkdownIt('commonmark').enable('table')
for path in sys.argv[1:]:
    with open(path, encoding='utf-8') as f:
        tokens = parser.parse(f.read())
    paragraphs = []
    in_paragraph = False
    for token in tokens:
        if token.type == 'paragraph_open':
            in_paragraph = True
        elif token.type == 'paragraph_close':
            in_paragraph = False
        elif in_paragraph and token.type == 'inline':
            parts = []
            for child in token.children or []:
                if child.type in ('text', 'softbreak', 'hardbreak'):
                    parts.append(child.content if child.type == 'text' else ' ')
                elif child.type == 'code_inline':
                    parts.append(' item ')
            text = re.sub(r'\s+', ' ', ''.join(parts)).strip()
            if text:
                paragraphs.append(text)
    sentences = []
    for paragraph in paragraphs:
        for sentence in re.split(r'(?<=[.!?])\s+(?=[A-Za-z])', paragraph):
            sentence = sentence.strip()
            if sentence and re.search(r'[.!?]$', sentence):
                sentences.append(sentence)
    counts = [len(re.findall(r"[A-Za-z]+(?:['’-][A-Za-z]+)*", s)) for s in sentences]
    prose = ' '.join(sentences)
    print(path, len(sentences), sum(counts), round(statistics.mean(counts), 1),
          sum(n > 25 for n in counts), textstat.flesch_kincaid_grade(prose),
          textstat.flesch_reading_ease(prose))
```

## 定性的な評価

- **CEFR の目安**: Git と CLI の知識がある開発者が導入判断に必要な情報を読む場合、README はおおむね B2 相当の読解力を持
  つ人を想定できる。ただし実読テストをしておらず、個々の読者の理解度や文書自体の等級を保証するものではない。詳細仕様は条
  件分岐やドメイン知識の負荷が高く、同じ読者でも読み直しが必要になりうる。
- **良い点**: 利用者向け 5 文書の文長は平均 12〜15 語程度で、指示の多くは動詞から始まる。例、箇条書き、参照先もあり、短
  く具体的に書くという両ガイドの方向におおむね沿っている。受動態があっても、主体より結果が重要な説明では問題とは限らな
  い。
- **情報密度**: [Message format の Header の説明](../references/message-format.md#header)には、 <!-- rumdl-disable-line MD089 -->
  複数の Rule と条件を一文にまとめた例がある。Rule ごとの箇条書きにすれば、短文にするだけでなく、対象と条件を照合しやすくなる。既定値や比較方法を落とさない
  ことが前提である。
- **専門用語**: `scissors`, `provenance`, `heuristic`, `schema` や `severity` は、単語だけでなく Git やこのツールの知識
  も必要とする。すべてを別の語に置き換えると正確さを損なうため、初出時の短い説明や既存のリファレンスへのリンクを優先す
  る。
- **導入時の読み順**: README のフィルタリングと無視の注意は重要だが、インストールとフックの説明にも重複している。#17 の
  ように要点をまとめて詳細にリンクする方が、意味を落とさずに探しやすさを改善できる。
- **見出し**: `Caveats and current limitations` は具体的な内容を示す自然な名詞句で、
  Google と Microsoft の見出しの方向性に合う。`Caveats` だけより内容は明確だが、
  単語自体になじみがない読者もいるため、より平易にするなら
  `Things to know and current limitations` も候補となる。

文長、受動態や用語の機械的な検出は、人が確認する箇所を探す補助として扱う。コード、コマンド、固有名や文脈による誤検出を確
認し、意味や技術的正確さを保つ。これらの指標から CEFR を換算したり、単独で読みやすさの合否を決めたりしない。推奨と必須を
分け、意味を保つ裁量を残す点は、[本リポジトリの設計方針](../../README.md#design-philosophy)とも整合する。 <!-- rumdl-disable-line MD089 -->

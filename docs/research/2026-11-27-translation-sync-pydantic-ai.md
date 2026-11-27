<!-- rumdl-disable MD013 -->

# 翻訳同期レビューにおける Pydantic AI の調査

調査日: 2026-11-27。翻訳同期レビューに使う Pydantic AI の文書と公開 wheel、GitHub などの一次資料を確認した。文書は更新されるため、以下の挙動は採用するパッケージ版で再確認する。

## 結論

Pydantic AI はモデルと認証を切り替えられる Python インターフェースを提供する。**Copilot サブスクリプションも、Pydantic AI の直接統合で利用可能と同プロジェクトは文書化している。** この統合は Copilot の OpenAI 互換 Chat Completions に直接接続する。ただし全プラン・全モデルで使えるという意味ではなく、GitHub がこの直接接続を外部アプリ向けに公式サポートしているか、適用条件を満たすかは確認できなかった。契約・組織ポリシーと実際の認証を別途確認する。[Pydantic AI: GitHub Copilot](https://ai.pydantic.dev/models/github-copilot/)

レビューでは Git 証拠をスクリプトが収集し、**モデルに実行ツールを渡さず**、人間が結果を確認する。複数プロバイダーへの対応と構造化出力を得る一方、Python 依存関係、認証、各社の機能差とデータ取り扱いを管理する必要がある。自動承認を避け、不確実性を人間に残す方針は、推奨と必須を分けて判断の余地を残す本プロジェクトの [設計方針](../../README.md#design-philosophy) と整合する。異常時にレビューを未完了とすることが、柔軟さとのトレードオフである。

## プロバイダー、認証、モデル

- `Agent('openai:<model>')`、`Agent('anthropic:<model>')`、`Agent('google:<model>')` などで選択でき、モデルは実行時にも指定できる。OpenAI は `OPENAI_API_KEY`、Anthropic は `ANTHROPIC_API_KEY`、Gemini API は `GOOGLE_API_KEY` が基本例である。別のクラウド認証や互換エンドポイントには明示的な provider/client が必要な場合がある。統一 API は、モデルの機能・設定・料金・データ利用条件の同一性を保証しない。[モデル一覧](https://ai.pydantic.dev/models/overview/)、[OpenAI](https://ai.pydantic.dev/models/openai/)、[Anthropic](https://ai.pydantic.dev/models/anthropic/)、[Google](https://ai.pydantic.dev/models/google/)
- モデル ID、プロバイダー、エンドポイントと必要な設定は利用者が明示的に選ぶのがよい。`ModelSettings` で `max_tokens`、`temperature`、`thinking` などを設定できるが、非対応設定は無視または警告され、値やツール・出力形式の対応もモデルごとに異なる。設定値を全社共通の保証としない。[モデル設定](https://ai.pydantic.dev/agents/#additional-configuration)、[モデルとプロファイル](https://ai.pydantic.dev/models/overview/#models-and-providers)
- フォールバックは別モデル・別提供者へ同じ証拠を送る可能性がある。可用性は上がるが送信先と料金も変わるため、明示的な許可なしに有効化しない。[フォールバック](https://ai.pydantic.dev/models/overview/#fallback-model)

### GitHub Copilot の直接統合

[Pydantic AI の Copilot 文書](https://ai.pydantic.dev/models/github-copilot/) と、PyPI の [`pydantic-ai-slim` 2.76.0 wheel](https://pypi.org/project/pydantic-ai-slim/2.76.0/#files) を確認した。wheel に `pydantic_ai/models/github_copilot.py`、`providers/github_copilot.py`、OAuth ヘルパーが含まれ、モデルは OpenAI Chat モデル、provider は OpenAI クライアントを使う。`pydantic-ai-slim[openai]` をインストールし、`github-copilot:<model>` または `GitHubCopilotModel` を指定する。調査時に一時環境の Python 3.13.13 で 2.76.0 の両クラスを import できた。**公開済み wheel の機能**であり、開発版文書だけの機能ではない。

- `GITHUB_COPILOT_API_KEY`、そのフォールバック `GITHUB_COPILOT_API_TOKEN`、`COPILOT_GITHUB_TOKEN`、または `GitHubCopilotProvider(api_key=...)` を使う。一般の `GH_TOKEN`、`GITHUB_TOKEN` は自動読取しない。Pydantic AI 文書では `gho_` OAuth ユーザートークンや、一部プランの Copilot API トークンが動作するとしている。自身の OAuth アプリの client ID で device flow を使うこともできるが、GitHub ログインの成功は Copilot の利用資格やそのアプリのトークン受理を保証しない。トークン更新・保存はアプリ側の責任である。
- fine-grained PAT について Pydantic AI は、Individual プランの既定 integration ID では 401 を受けたと報告する。classic PAT は非対応としている。[Pydantic AI: 認証](https://ai.pydantic.dev/models/github-copilot/#configuration)。
- 利用できるモデル ID はプラン、組織ポリシー、`Copilot-Integration-Id` に依存する。Pydantic AI の既定 ID は `vscode-chat` で変更可能である。文書の `/models` 照会に同じ ID とトークンを使い、各モデルの `supported_endpoints` に `/chat/completions` があるか確認する。トークンを画面・ログ・シェル履歴に漏らさない。ID が一覧にあっても API が異なれば使えない。現行統合は Copilot の Responses、Messages、realtime、embeddings を実装していない。モデル一覧や reasoning 設定の受理範囲も変化する。
- Pydantic AI は Copilot の AI credits とモデル別トークン料金を説明するが、実際の課金・上限はプランと経路を確認する。GitHub の [現行料金表](https://docs.github.com/en/copilot/reference/copilot-billing/models-and-pricing) と [旧 premium request 課金](https://docs.github.com/en/copilot/concepts/billing/copilot-requests) は区別する。利用する統合の課金条件を個別に確認する。文書上、Copilot 経路のコストとコンテキスト窓の情報は取得できず、コスト制限や事前見積もりをそれだけに頼れない。

Pydantic AI 自身による実測・対応説明は確認できたが、GitHub によるこの第三者クライアント、その既定の `vscode-chat` integration ID、任意の OAuth アプリの直接利用に対する明示的なサポート・許諾は確認できなかった。動作するトークンやモデル一覧が得られても、将来の互換性や利用規約への適合は証明されない。特に組織利用では GitHub と管理者に確認する。

## ツールと安全境界

Pydantic AI は `@agent.tool` や `@agent.tool_plain` で Python 関数を登録し、引数スキーマをモデルに渡せる。MCP やプロバイダー側の native tools も別途設定できる。ツールを登録しない場合は、通常の関数ツールをモデルが呼ぶ経路を作らずに済む。構造化出力のための output tool はデータを返す仕組みであり、任意のシェルを与えることとは区別する。[function tools](https://ai.pydantic.dev/tools/)、[構造化出力](https://ai.pydantic.dev/output/)

- 将来必要な場合も `git(command: str)` や任意シェルを渡さない。`shell=False` だけでは任意の Git サブコマンドやオプションの副作用を防げない。例えば指定済みの対・検証済み SHA に限定した `read_document` / `read_diff` のような固定操作を実装し、コマンド・オプション・パスをホスト側で決める。`--`、literal pathspec、正規化と許可リスト、ファイル種別・サイズ検査を組み合わせる。Git の外部 diff や textconv はプログラムを起動し得るため無効化し、環境変数や Git 設定も制御する。[Git diff](https://git-scm.com/docs/git-diff)、[Git attributes](https://git-scm.com/docs/gitattributes)
- 実行ツールの登録、MCP、native tools、出力関数、計測はそれぞれ独立に監査する。承認要求やツール回数制限は OS サンドボックスではない。必要に応じて読み取り専用の隔離環境、最小権限、送信先制限を併用する。動的な証拠取得はコンテキスト量を減らし得るが、モデルが必要な証拠を取得し損ねたり、より多くのデータを送ったりするリスクが増える。
- 文書、差分、ファイル名、コミットメッセージ、ツール結果は攻撃者が指示を埋め込める非信頼データである。「指示ではなく証拠」と明記するだけでは注入を完全に防げない。結果も非信頼として扱い、根拠を人間が検証する。ツールの権限を絞れば注入成功時の被害を抑えられるが、レビューの誤誘導は残る。[OWASP: Prompt Injection](https://genai.owasp.org/llmrisk/llm01-prompt-injection/)
- 送信前に証拠をプレビューし、秘密や個人情報、ライセンス上の制約、提供者の保存・学習・地域・組織ポリシーを確認する。プロンプト、ツール結果、エラー、モデル応答、ログ・トレースにも情報が残り得る。Logfire 等の計測を有効化するなら送信先と内容を確認し、必要に応じ `include_content=False` を使う。これはプロバイダーへのモデル入力の送信自体を防ぐものではない。[Logfire: 内容の除外](https://ai.pydantic.dev/integrations/logfire/#excluding-prompts-and-completions)

## 上限、再試行、出力、時間

- `UsageLimits(request_limit=..., tool_calls_limit=..., input_tokens_limit=..., output_tokens_limit=...)` などで実行を制限できる。`tool_calls_limit` は成功したツール呼び出し数を数え、並列呼び出し全体が上限を超える場合は実行前に止める。失敗や試行の厳密な総数制限ではない。`request_limit` も SDK や transport の再試行、フォールバック等の実際のネットワーク試行数をすべて数えるものではない。トークンや料金の計測は提供者が返す情報にも依存する。[usage limits](https://ai.pydantic.dev/agents/#usage-limits)、[UsageLimits API](https://ai.pydantic.dev/api/usage/#pydantic_ai.usage.UsageLimits)
- ツール引数・出力の検証失敗に対するモデルへの再指示と、通信層・各提供者 SDK の再試行は別予算である。前者は `retries`、後者は SDK、transport 等で設定する。層を重ねると試行回数と遅延が乗算される。例えば OpenAI SDK の既定は 2 回の再試行である。再試行の担当層と各上限を明示し、認証失敗、制限超過、タイムアウトや検証失敗は未完了として扱う。[再試行の層](https://ai.pydantic.dev/retries/#the-layers)、[再試行の乗算](https://ai.pydantic.dev/retries/#retry-multiplication)
- `output_type` に Pydantic モデルを渡して、例えば指摘ごとのパス・引用・説明と、未確認事項・レビュー未完了理由を受け取れる。通常の構造化出力には tool output、明示的な `NativeOutput` には提供者の JSON Schema 機能を使うが、対応やツールとの併用制約はモデルごとに異なる。検証成功は形式の保証であって、引用の正確さや差分の網羅性、翻訳の意味的一致の証明ではない。空の指摘を承認扱いにしない。[出力](https://ai.pydantic.dev/output/)
- `ModelSettings.timeout` は対応モデルの通信試行単位、`request_timeout` はそのモデル要求の SDK・transport 再試行を含む期限、`agent.run(..., timeout=...)` / `run_sync` の `timeout` は実行全体の期限である。フォールバック先は別の request deadline を得る。function tool の timeout は別に設定する。同期 `def` ツールはタイムアウト後もワーカースレッド内で完了まで走り得るため、子プロセスを含む強制停止の保証として使わない。キャンセル後にリモート側の生成・課金が直ちに止まることも前提にしない。[タイムアウト](https://ai.pydantic.dev/timeouts/)

## 依存関係と実行環境

PEP 723 のインラインメタデータは依存関係の宣言であり、Python 自体や実行器ではない。スクリプトでは OpenAI、Anthropic、Google および Copilot の各プロバイダーに必要な依存関係を宣言する。他のプロバイダーには追加の extra が必要な場合がある。[PEP 723 仕様](https://packaging.python.org/en/latest/specifications/inline-script-metadata/)、[インストール](https://ai.pydantic.dev/install/)

```python
# /// script
# requires-python = ">=3.11"
# dependencies = ["pydantic-ai-slim[openai,anthropic,google]==2.76.0"]
# ///
```

調査時の PyPI 公開版 `pydantic-ai-slim` 2.76.0 は Python 3.11 以上を要する。[PyPI JSON](https://pypi.org/pypi/pydantic-ai-slim/2.76.0/json)。`uv run script.py` は宣言に基づく環境と Python の探索を扱うが、uv の導入が必要である。[uv](https://docs.astral.sh/uv/guides/scripts/#declaring-script-dependencies)。トップレベルだけのバージョン固定は推移的依存関係の完全固定ではない。固定は再現性を改善する一方、更新・セキュリティ保守の負担を生む。

## 現行スクリプトと未検証事項

`check_translation_sync.py` はクリーンな作業ツリーと履歴、対ごとの baseline・HEAD の祖先関係を検査し、対象文書の baseline/HEAD、ログ、差分、変更パスと非 Markdown の現在の内容を組み立てる。他の Markdown は内容・差分を省略し、パスを示す。`--print-prompt` は送信前の確認用で、通常の `review` は対ごとに選択したモデルへ証拠を送り、構造化された結果を検証する。`acknowledge` は人間が指定した SHA を記録し、レビューは baseline を進めない。[スクリプト](../../scripts/check_translation_sync.py)

証拠の収集範囲・省略の明示、送信前プレビュー、HEAD の固定、対ごとの人間による確認と明示的な `acknowledge` を維持する。制限・モデル差・コンテキスト超過・拒否・部分結果・エラーを成功や承認に変換せず、未完了として表示する。

今回の実測は公開 wheel の内容確認と、一時環境での 2.76.0 の import に限る。Copilot を含むいずれのプロバイダーにも実接続しておらず、実アカウントの資格、モデル一覧、トークン種別、課金、組織ポリシー、構造化出力や制限・中断の実挙動、翻訳レビュー品質は検証していない。利用前に承認済みの送信先と非機密のサンプルで確認する。GitHub の直接 API のサポート・利用条件については上記の不確実性が残る。

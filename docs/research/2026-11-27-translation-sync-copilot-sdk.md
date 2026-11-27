<!-- rumdl-disable MD013 -->

# 翻訳同期に使う Python 依存関係と Copilot SDK

調査日: 2026-11-27。PyPI の公開版と一次資料を確認した。SDK の API は公開版 2.0.3 に対応するソースを参照し、開発中の `main` や旧版の例とは区別する。結論として、pip だけでインライン依存関係を**インストール**できるが、スクリプトの実行と環境管理は別途必要である。SDK は CLI をサブプロセスとして呼び出す方式の代替になり得るが、Copilot の実行ランタイムや認証を不要にするものではない。

## pip と PEP 723

- pip **26.0**（2026-01-30）から `--requirements-from-script` が使える。調査時の最新安定版は **26.3**（2026-11-02、Python 3.10 以上）である。[pip changelog](https://pip.pypa.io/en/stable/news/#v26-0)、[PyPI: pip](https://pypi.org/project/pip/26.3/)。
- `# /// script` ブロックの TOML に `dependencies` と `requires-python` を書く。[PyPA 仕様](https://packaging.python.org/en/latest/specifications/inline-script-metadata/) による。pip 26.3 は指定された対象 Python のバージョンも検査し、不一致なら失敗する。`--ignore-requires-python` で検査を抑制できるが、実際の互換性は保証されない。オプションに指定できるスクリプトは一つである。[pip 26.3 実装](https://github.com/pypa/pip/blob/191854fba83d1c69534f974564af85632372d995/src/pip/_internal/cli/req_command.py#L476-L505)。

例えば `sync.py` の冒頭に以下を置く。バージョン固定は再現性と更新作業のトレードオフである。

```python
# /// script
# requires-python = ">=3.11"
# dependencies = ["microsoft-copilot-sdk==2.0.3"]
# ///
```

Unix 系での実行例は次のとおり。`python3` は 3.11 以上とする。pip が古い環境でも、最初に pip を更新する。仮想環境は、クリーンな作業ツリーを要求する翻訳同期スクリプトに影響しないよう、Git の追跡対象外である `.scratch/` に置く。

```sh
python3 -m venv .scratch/translation-sync-venv
.scratch/translation-sync-venv/bin/python -m pip install --upgrade 'pip>=26.0'
.scratch/translation-sync-venv/bin/python -m pip install --requirements-from-script scripts/check_translation_sync.py
.scratch/translation-sync-venv/bin/python scripts/check_translation_sync.py review --pair readme
```

pip は環境の作成やスクリプトの実行を自動では行わない。[pip install のオプション](https://pip.pypa.io/en/stable/cli/pip_install/#cmdoption-requirements-from-script)。別の選択肢として、[uv](https://docs.astral.sh/uv/guides/scripts/#declaring-script-dependencies) は `uv run sync.py` でインライン依存関係を扱い、必要な Python の探索も行う。ただし uv 自体が別途必要である。

## SDK、ランタイムと認証

- 調査時の現行パッケージは [`microsoft-copilot-sdk` 2.0.3](https://pypi.org/project/microsoft-copilot-sdk/2.0.3/)（Python 3.11 以上）。旧 [`github-copilot-sdk`](https://pypi.org/project/github-copilot-sdk/) は非推奨で、2.x は破壊的変更を伴う。古い `CopilotClient()` や `on_permission_request=...` の例をそのまま 2.x に適用しない。[v2 移行ガイド](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/docs/v2-breaking-changes.md)。
- SDK は Copilot CLI と同じエージェント基盤を JSON-RPC 経由で利用するが、**別途 CLI のインストールは必須ではない**。2.x の既定はネイティブランタイムのプロセス内ロードで、Python SDK は対応バンドルを初回利用時にダウンロードする。`python -m copilot download-runtime --in-process` で事前取得も可能。明示的に子プロセスや外部サーバーを選ぶこともできる。したがって「CLI コマンドの起動」は避けられても、対応ランタイムとその配布・ネットワーク要件は残る。[SDK README](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/README.md)、[Python README](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/python/README.md#runtime)。
- 通常は Copilot の利用資格が必要で、CLI で保存したログイン、明示的なトークン、環境変数などを使える。環境変数の優先順は `COPILOT_GITHUB_TOKEN`、`GH_TOKEN`、`GITHUB_TOKEN`。トークンを設定するだけで、そのトークンや組織に Copilot の利用権限が付くわけではない。BYOK は対応モデル提供者の鍵を使い、GitHub 認証を迂回できる。利用量・課金条件も確認する。[認証](https://docs.github.com/en/copilot/how-tos/copilot-sdk/auth/authenticate)、[SDK FAQ](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/README.md#faq)。GitHub Docs の一部には v1 の Python コード例が残っているため、2.x の構文はバージョン固定の SDK 資料を優先する。

## Python API と読み取り専用の境界

以下は SDK 2.0.3 の API に基づく最小例である。翻訳対象をホスト側で読み、文字列として渡す方式を想定する。ツールを空にし、万一の許可要求も拒否する。実サービスへの接続例であり、認証と利用可能なモデルが必要である。

```python
import asyncio

from copilot import CopilotClient
from copilot.rpc import PermissionDecisionReject
from copilot.session_events import AssistantMessageData


def deny(request, invocation):
    return PermissionDecisionReject(feedback="Tool access is disabled")


async def main():
    async with CopilotClient(mode="coding-agent") as client:
        async with await client.create_session(
            model="auto",
            tools={"available_tools": []},
            permissions={"on_request": deny},
        ) as session:
            response = await session.send_and_wait(
                "Translate this text into Japanese: Hello, world.", timeout=120
            )
            if response and isinstance(response.data, AssistantMessageData):
                print(response.data.content)
            else:
                raise RuntimeError("No assistant message returned")


asyncio.run(main())
```

`send_and_wait` は最終応答イベントまたは `None` を返す。既定の待機上限は 60 秒で、タイムアウトはエージェントの処理自体を停止しない。中断が必要なら `session.abort()` を検討する。ストリーミングには `session.on(...)` とイベントを使う。[Python API](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/python/copilot/session.py#L2555-L2615)。

**許可設定は OS の読み取り専用サンドボックスではない。** `PermissionHandler.approve_all` は書き込みやシェルを含む要求を承認するため使わない。ハンドラーを省略した場合は要求がイベントとして保留され、未処理のまま待機し得る。必要な読み取りツールを許す場合も、シェル、書き込み、外部 URL、カスタムツールや MCP の副作用を個別に評価する。`skip_permission` を指定したカスタムツールなどの例外にも注意し、強い保証が必要なら OS 権限、隔離環境、ネットワーク制限を併用する。プロセス内ランタイムはサンドボックスでもクラッシュ分離でもない。[権限処理](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/python/README.md#permission-handling)、[ランタイム方式](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/docs/setup/in-process-runtime.md)。

## 指示、MCP、ライフサイクル

- `system_message` で追加指示を与えられる。`replace` は既定の安全上のガードレールも取り除くため、単なる翻訳指示には避ける。`coding-agent` モードの指示ファイルの扱いは CLI と同一と決めつけず、作業ディレクトリと設定を明示し、必要に応じ `await session.rpc.instructions.get_sources()` で確認する。`instructions={"skip_custom_instructions": True}` でカスタム指示を抑制できる。CLI には `AGENTS.md` や `.github/copilot-instructions.md` の発見規則がある。[Python README](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/python/README.md#system-message-customization)、[SDK 互換性](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/docs/troubleshooting/compatibility.md)、[CLI の指示](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions)。
- MCP は `create_session(mcp={"servers": {...}})` でローカル stdio またはリモート HTTP/SSE サーバーを指定でき、サーバーごとに `tools` を絞れる。既存の設定やプラグインが発見される場合もあるため、空の辞書だけで全 MCP が無効になると仮定しない。上の例ではセッションのツール許可リストを空にしている。[MCP ガイド](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/docs/features/mcp.md)、[ツールのフィルター](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/python/README.md#copilotclient)。
- `async with` はセッションを切断しクライアントを停止するが、セッションデータの削除とは異なる。保存データを消すには `client.delete_session(session_id)` が別途必要である。所有する子プロセスは停止時に終了するが、外部管理ランタイムは終了しない。SDK の API とランタイムは対応バージョンを揃え、CLI 専用 UI やフラグに SDK の完全な等価物があるとは仮定しない。[セッションの永続化](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/docs/features/session-persistence.md#disconnecting-from-a-session-disconnect)、[互換性](https://github.com/microsoft/copilot-sdk/blob/5671812ecd4ce58ee040e9a41d98da2f849c8095/docs/troubleshooting/compatibility.md)。

## 確認範囲と設計上の判断

一時仮想環境の Python 3.15、pip 26.3 で上記形式のメタデータから SDK 2.0.3 をインストールし、`copilot` の import とバージョンを確認した。API のシグネチャも公開 wheel で確認した。Copilot への実接続、権限拒否、MCP・指示の実行時挙動は検証していない。

SDK は構造化イベントと明示的な権限制御を得られる一方、外部ランタイム、認証、モデル利用料、バージョン移行の管理が増える。翻訳だけなら入力をホスト側で読み、ツールを無効にして出力を検査する方式が制御しやすい。リポジトリの設計哲学は推奨と必須の制限を分け、意味を保つ裁量を残している。翻訳同期でも機械的な制限と人間による意味の確認を分けることは、その考え方に沿う。ただし、この方式自体は翻訳の品質や安全性を保証しない。

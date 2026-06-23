"""gemini_challenger 单元测试（mock OpenAI API，不需要真实 key）。

底层模型 = OpenAI GPT-5.5（"gemini" 是异质质询工位的角色名，非模型名）。
异质源 2026-06-23 从 Google Gemini 迁移到 OpenAI GPT-5.5（编排者指令）。

覆盖：
1. GeminiChallenger 初始化（key 缺失报错）
2. challenge() 调用链和返回结构
3. verify() 调用链和返回结构
4. decide() 调用链和返回结构
5. derive() 调用链和返回结构
6. 模块级便捷函数
7. 主模型 APIError 时降级到 fallback
"""

from __future__ import annotations

import asyncio
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from newchan.gemini_challenger import (
    ChallengeResult,
    GeminiChallenger,
)


class _FakeAPIError(Exception):
    """模拟 openai.APIError（无需构造真实 request/body）。"""


def _mock_response(text: str | None) -> MagicMock:
    """构造一个 OpenAI Responses API 风格的 mock 响应对象。"""
    r = MagicMock()
    r.output_text = text or ""
    r.output = []  # output_text 为空时，_extract_text 经此返回 ""
    return r


def _wire(mock_openai: MagicMock) -> MagicMock:
    """配置 mock_openai：真实异常类 + 返回 responses.create 句柄。"""
    mock_openai.APIError = _FakeAPIError
    return mock_openai.OpenAI.return_value.responses.create


class TestGeminiChallengerInit:
    """初始化行为。"""

    def test_missing_key_raises(self) -> None:
        with patch.dict("os.environ", {}, clear=True):
            with pytest.raises(ValueError, match="OPENAI_API_KEY"):
                GeminiChallenger(api_key="")

    def test_explicit_key(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            _wire(mock_openai)
            GeminiChallenger(api_key="test-key-123")
            mock_openai.OpenAI.assert_called_once_with(api_key="test-key-123")

    def test_env_key(self) -> None:
        with (
            patch.dict("os.environ", {"OPENAI_API_KEY": "env-key-456"}),
            patch("newchan.gemini_challenger.openai") as mock_openai,
        ):
            _wire(mock_openai)
            GeminiChallenger()
            mock_openai.OpenAI.assert_called_once_with(api_key="env-key-456")


class TestChallenge:
    """challenge() 方法。"""

    def test_returns_challenge_result(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("无否定。检查了定义一致性。")

            c = GeminiChallenger(api_key="test")
            result = c.challenge("中枢定义", context="3段重叠")

            assert isinstance(result, ChallengeResult)
            assert result.mode == "challenge"
            assert result.subject == "中枢定义"
            assert result.response == "无否定。检查了定义一致性。"
            assert result.model == "gpt-5.5-pro"

    def test_calls_api_with_correct_model(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("ok")

            c = GeminiChallenger(api_key="test", model="gpt-custom")
            result = c.challenge("test subject")

            assert create.call_args.kwargs["model"] == "gpt-custom"
            assert result.model == "gpt-custom"

    def test_uses_xhigh_reasoning(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("ok")

            c = GeminiChallenger(api_key="test")
            c.challenge("test subject")

            assert create.call_args.kwargs["reasoning"] == {"effort": "xhigh"}

    def test_empty_response_text(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response(None)

            c = GeminiChallenger(api_key="test")
            result = c.challenge("test")
            assert result.response == ""


class TestVerify:
    """verify() 方法。"""

    def test_returns_verify_result(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("成立。推理链有效。")

            c = GeminiChallenger(api_key="test")
            result = c.verify("ZG > ZD 恒成立", context="三段重叠法")

            assert isinstance(result, ChallengeResult)
            assert result.mode == "verify"
            assert result.subject == "ZG > ZD 恒成立"
            assert result.response == "成立。推理链有效。"


class TestDecide:
    """decide() 方法。"""

    def test_returns_decide_result(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("选择方案A。推理：一致性更高。")

            c = GeminiChallenger(api_key="test")
            result = c.decide("是否采用递归级别", context="当前有两种方案")

            assert isinstance(result, ChallengeResult)
            assert result.mode == "decide"
            assert result.subject == "是否采用递归级别"
            assert result.response == "选择方案A。推理：一致性更高。"
            assert result.model == "gpt-5.5-pro"

    def test_uses_orchestrator_system_prompt(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("决策结果")

            c = GeminiChallenger(api_key="test")
            c.decide("test subject")

            kwargs = create.call_args.kwargs
            assert kwargs["reasoning"] == {"effort": "xhigh"}
            assert "编排者代理" in kwargs["instructions"]

    def test_empty_response_text(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response(None)

            c = GeminiChallenger(api_key="test")
            result = c.decide("test")
            assert result.response == ""

    def test_fallback_on_api_error(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.side_effect = [
                _FakeAPIError("primary down"),
                _mock_response("fallback decide"),
            ]

            c = GeminiChallenger(api_key="test")
            result = c.decide("test subject")

            assert result.model == "gpt-5.5"
            assert result.response == "fallback decide"
            assert result.mode == "decide"


class TestDerive:
    """derive() 方法。"""

    def test_returns_derive_result(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("### 4. Conclusion\nPROVEN. Q.E.D.")

            c = GeminiChallenger(api_key="test")
            result = c.derive(
                "笔的递归完备性", context="缠论公理集",
                domain="Formalized Chanlun System",
            )

            assert isinstance(result, ChallengeResult)
            assert result.mode == "derive"
            assert result.subject == "笔的递归完备性"
            assert result.response == "### 4. Conclusion\nPROVEN. Q.E.D."
            assert result.model == "gpt-5.5-pro"

    def test_uses_derive_system_prompt(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("推导结果")

            c = GeminiChallenger(api_key="test")
            c.derive("test statement", domain="Topology")

            kwargs = create.call_args.kwargs
            assert kwargs["reasoning"] == {"effort": "xhigh"}
            assert "Formal Mathematical Proof Engine" in kwargs["instructions"]

    def test_empty_response(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response(None)

            c = GeminiChallenger(api_key="test")
            result = c.derive("test")
            assert result.response == ""

    def test_fallback_on_api_error(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.side_effect = [
                _FakeAPIError("primary down"),
                _mock_response("fallback derive"),
            ]

            c = GeminiChallenger(api_key="test")
            result = c.derive("test statement")

            assert result.model == "gpt-5.5"
            assert result.response == "fallback derive"
            assert result.mode == "derive"

    def test_default_domain(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("result")

            c = GeminiChallenger(api_key="test")
            c.derive("1+1=2")

            prompt = create.call_args.kwargs["input"]
            assert "General Mathematics" in prompt


class TestModuleLevelFunctions:
    """模块级 challenge()/verify()/decide()/derive() 便捷函数。"""

    def test_challenge_auto_init(self) -> None:
        import newchan.gemini_challenger as mod

        mod._default_challenger = None  # 重置 singleton

        with patch.object(mod, "GeminiChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.challenge.return_value = ChallengeResult(
                mode="challenge", subject="x", response="y", model="m",
            )
            MockClass.return_value = mock_instance

            result = mod.challenge("x")
            MockClass.assert_called_once()
            mock_instance.challenge.assert_called_once_with("x", "")
            assert result.response == "y"

        mod._default_challenger = None  # 清理

    def test_verify_auto_init(self) -> None:
        import newchan.gemini_challenger as mod

        mod._default_challenger = None

        with patch.object(mod, "GeminiChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.verify.return_value = ChallengeResult(
                mode="verify", subject="a", response="b", model="m",
            )
            MockClass.return_value = mock_instance

            result = mod.verify("a", "ctx")
            mock_instance.verify.assert_called_once_with("a", "ctx")
            assert result.response == "b"

        mod._default_challenger = None

    def test_decide_auto_init(self) -> None:
        import newchan.gemini_challenger as mod

        mod._default_challenger = None

        with patch.object(mod, "GeminiChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.decide.return_value = ChallengeResult(
                mode="decide", subject="q", response="r", model="m",
            )
            MockClass.return_value = mock_instance

            result = mod.decide("q", "ctx")
            mock_instance.decide.assert_called_once_with("q", "ctx")
            assert result.mode == "decide"
            assert result.response == "r"

        mod._default_challenger = None

    def test_derive_auto_init(self) -> None:
        import newchan.gemini_challenger as mod

        mod._default_challenger = None

        with patch.object(mod, "GeminiChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.derive.return_value = ChallengeResult(
                mode="derive", subject="s", response="proof", model="m",
            )
            MockClass.return_value = mock_instance

            result = mod.derive("s", "axioms", "Topology")
            mock_instance.derive.assert_called_once_with("s", "axioms", "Topology")
            assert result.mode == "derive"
            assert result.response == "proof"

        mod._default_challenger = None


class TestToolsLoop:
    """OpenAI Responses API + MCP 手动 function-calling 循环（mock 管线验证 L1）。"""

    def test_tool_loop_dispatches_and_extracts_chain(self) -> None:
        from newchan.gemini.engine import call_with_tools_and_fallback

        # 第一轮：reasoning summary + 一个 function_call
        first = SimpleNamespace(
            id="resp_1",
            output=[
                SimpleNamespace(
                    type="reasoning",
                    summary=[SimpleNamespace(text="先读取符号定义")],
                ),
                SimpleNamespace(
                    type="function_call",
                    name="find_symbol",
                    arguments='{"name": "ZhongShu"}',
                    call_id="call_abc",
                ),
            ],
        )
        # 第二轮：无 function_call，给出最终文本
        second = SimpleNamespace(
            id="resp_2",
            output_text="结论：中枢定义自洽，无否定。",
            output=[],
        )

        async_client = MagicMock()
        async_client.responses.create = AsyncMock(side_effect=[first, second])

        bridge = MagicMock()
        bridge.get_tools = AsyncMock(return_value=[
            SimpleNamespace(
                name="find_symbol", description="locate symbol",
                parameters={"type": "object", "properties": {}},
            ),
        ])
        bridge.call_tool = AsyncMock(
            return_value=SimpleNamespace(content="class ZhongShu: ...")
        )

        fake_openai = SimpleNamespace(APIError=_FakeAPIError)

        text, model, calls, chain = asyncio.run(
            call_with_tools_and_fallback(
                async_client, "gpt-5.5-pro", "质询中枢定义", "xhigh",
                bridge, 20, "system", fake_openai,
            )
        )

        assert text == "结论：中枢定义自洽，无否定。"
        assert model == "gpt-5.5-pro"
        assert calls == ("find_symbol(name='ZhongShu')",)
        # 推理链含 thought + tool_call + tool_result 三类
        types_seen = {step["type"] for step in chain}
        assert types_seen == {"thought", "tool_call", "tool_result"}
        # 工具被实际派发，且工具输出经 function_call_output 回填
        bridge.call_tool.assert_awaited_once_with("find_symbol", {"name": "ZhongShu"})
        second_call_input = async_client.responses.create.call_args_list[1].kwargs["input"]
        assert second_call_input[0]["type"] == "function_call_output"
        assert second_call_input[0]["call_id"] == "call_abc"

    def test_tool_loop_falls_back_on_api_error(self) -> None:
        from newchan.gemini.engine import call_with_tools_and_fallback

        ok = SimpleNamespace(id="r", output_text="ok", output=[])
        async_client = MagicMock()
        async_client.responses.create = AsyncMock(
            side_effect=[_FakeAPIError("pro down"), ok]
        )
        bridge = MagicMock()
        bridge.get_tools = AsyncMock(return_value=[])
        bridge.call_tool = AsyncMock()
        fake_openai = SimpleNamespace(APIError=_FakeAPIError)

        text, model, calls, chain = asyncio.run(
            call_with_tools_and_fallback(
                async_client, "gpt-5.5-pro", "q", "xhigh",
                bridge, 20, "system", fake_openai,
            )
        )
        assert text == "ok"
        assert model == "gpt-5.5"  # 降级到 fallback


class TestFallback:
    """主模型 APIError 时降级到 fallback。"""

    def test_fallback_on_api_error(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.side_effect = [
                _FakeAPIError("primary down"),
                _mock_response("fallback response"),
            ]

            c = GeminiChallenger(api_key="test")
            result = c.challenge("test subject")

            assert result.model == "gpt-5.5"
            assert result.response == "fallback response"
            # 主模型 1 次 + fallback 1 次（单次尝试，无重试循环）
            assert create.call_count == 2

    def test_no_fallback_when_primary_works(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.return_value = _mock_response("primary ok")

            c = GeminiChallenger(api_key="test")
            result = c.challenge("test")

            assert result.model == "gpt-5.5-pro"
            assert result.response == "primary ok"
            assert create.call_count == 1

    def test_both_fail_raises(self) -> None:
        with patch("newchan.gemini_challenger.openai") as mock_openai:
            create = _wire(mock_openai)
            create.side_effect = [
                _FakeAPIError("primary down"),
                _FakeAPIError("fallback down"),
            ]

            c = GeminiChallenger(api_key="test")
            with pytest.raises(_FakeAPIError):
                c.challenge("test")

"""gemini_challenger 单元测试（mock API，不需要真实 key）

覆盖：
1. GeminiChallenger 初始化（key 缺失报错）
2. challenge() 调用链和返回结构
3. verify() 调用链和返回结构
4. decide() 调用链和返回结构
5. 模块级便捷函数
6. CLI 入口参数解析
"""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest

from google.genai.errors import ServerError

from newchan.gemini_challenger import (
    ChallengeResult,
    GeminiChallenger,
)


class TestGeminiChallengerInit:
    """初始化行为。"""

    def test_missing_key_raises(self) -> None:
        with patch.dict("os.environ", {}, clear=True):
            with pytest.raises(ValueError, match="GOOGLE_API_KEY"):
                GeminiChallenger(api_key="")

    def test_explicit_key(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            c = GeminiChallenger(api_key="test-key-123")
            mock_genai.Client.assert_called_once_with(api_key="test-key-123")

    def test_env_key(self) -> None:
        with (
            patch.dict("os.environ", {"GOOGLE_API_KEY": "env-key-456"}),
            patch("newchan.gemini_challenger.genai") as mock_genai,
        ):
            c = GeminiChallenger()
            mock_genai.Client.assert_called_once_with(api_key="env-key-456")


class TestChallenge:
    """challenge() 方法。"""

    def test_returns_challenge_result(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = "无否定。检查了定义一致性。"
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test")
            result = c.challenge("中枢定义", context="3段重叠")

            assert isinstance(result, ChallengeResult)
            assert result.mode == "challenge"
            assert result.subject == "中枢定义"
            assert result.response == "无否定。检查了定义一致性。"
            assert result.model == "gemini-3-pro-preview"

    def test_calls_api_with_correct_model(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = "ok"
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test", model="gemini-custom")
            result = c.challenge("test subject")

            call_args = mock_genai.Client.return_value.models.generate_content.call_args
            assert call_args.kwargs["model"] == "gemini-custom"
            assert result.model == "gemini-custom"

    def test_empty_response_text(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = None
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test")
            result = c.challenge("test")
            assert result.response == ""


class TestVerify:
    """verify() 方法。"""

    def test_returns_verify_result(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = "成立。推理链有效。"
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test")
            result = c.verify("ZG > ZD 恒成立", context="三段重叠法")

            assert isinstance(result, ChallengeResult)
            assert result.mode == "verify"
            assert result.subject == "ZG > ZD 恒成立"
            assert result.response == "成立。推理链有效。"


class TestDecide:
    """decide() 方法。"""

    def test_returns_decide_result(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = "选择方案A。推理：一致性更高。"
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test")
            result = c.decide("是否采用递归级别", context="当前有两种方案")

            assert isinstance(result, ChallengeResult)
            assert result.mode == "decide"
            assert result.subject == "是否采用递归级别"
            assert result.response == "选择方案A。推理：一致性更高。"
            assert result.model == "gemini-3-pro-preview"

    def test_uses_orchestrator_system_prompt(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = "决策结果"
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test")
            c.decide("test subject")

            # 验证 GenerateContentConfig 被调用时传入了正确的 temperature
            config_call = mock_genai.types.GenerateContentConfig.call_args
            assert config_call.kwargs["temperature"] == 0.2

    def test_empty_response_text(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = None
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test")
            result = c.decide("test")
            assert result.response == ""

    def test_fallback_on_503(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_client = mock_genai.Client.return_value

            mock_503 = ServerError(503, {"error": {"message": "unavailable"}})
            mock_ok = MagicMock()
            mock_ok.text = "fallback decide"
            mock_client.models.generate_content.side_effect = [
                mock_503,
                mock_ok,
            ]

            c = GeminiChallenger(api_key="test")
            result = c.decide("test subject")

            assert result.model == "gemini-2.5-pro"
            assert result.response == "fallback decide"
            assert result.mode == "decide"


class TestModuleLevelFunctions:
    """模块级 challenge()/verify()/decide() 便捷函数。"""

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


class TestFallback:
    """主模型 503 时降级到 fallback。"""

    def test_fallback_on_503(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_client = mock_genai.Client.return_value

            # 第一次调用（主模型）抛 503
            mock_503 = ServerError(503, {"error": {"message": "unavailable"}})
            # 第二次调用（fallback）成功
            mock_ok = MagicMock()
            mock_ok.text = "fallback response"
            mock_client.models.generate_content.side_effect = [
                mock_503,
                mock_ok,
            ]

            c = GeminiChallenger(api_key="test")
            result = c.challenge("test subject")

            assert result.model == "gemini-2.5-pro"
            assert result.response == "fallback response"
            assert mock_client.models.generate_content.call_count == 2

    def test_no_fallback_when_primary_works(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_response = MagicMock()
            mock_response.text = "primary ok"
            mock_genai.Client.return_value.models.generate_content.return_value = (
                mock_response
            )

            c = GeminiChallenger(api_key="test")
            result = c.challenge("test")

            assert result.model == "gemini-3-pro-preview"
            assert result.response == "primary ok"

    def test_both_fail_raises(self) -> None:
        with patch("newchan.gemini_challenger.genai") as mock_genai:
            mock_503 = ServerError(503, {"error": {"message": "unavailable"}})
            mock_genai.Client.return_value.models.generate_content.side_effect = [
                mock_503,
                mock_503,
            ]

            c = GeminiChallenger(api_key="test")
            with pytest.raises(ServerError):
                c.challenge("test")

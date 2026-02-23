"""codex_challenger 单元测试（mock API，不需要真实 key）

覆盖：
1. CodexChallenger 初始化（key 缺失报错）
2. review() 调用链和返回结构
3. diagnose() 调用链和返回结构
4. decide() 调用链和返回结构
5. fallback (APIError → 降级)
6. CLI 参数解析
7. 模块级便捷函数
8. missing key raises ValueError
9. ReviewResult.to_markdown() 格式
10. CLI 持久化文件写入
"""

from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

import openai as _real_openai

from newchan.codex.modes import (
    CodexChallenger,
    ReviewResult,
)


# ── Helper: 构建 mock response ──

def _make_mock_response(text: str) -> MagicMock:
    """构建模拟 Responses API 返回对象。"""
    text_block = MagicMock()
    text_block.type = "output_text"
    text_block.text = text

    message_item = MagicMock()
    message_item.type = "message"
    message_item.content = [text_block]

    response = MagicMock()
    response.output = [message_item]
    return response


def _make_empty_response() -> MagicMock:
    """构建空响应。"""
    response = MagicMock()
    response.output = []
    return response


class TestCodexChallengerInit:
    """初始化行为。"""

    def test_missing_key_raises(self) -> None:
        with patch.dict("os.environ", {}, clear=True):
            with pytest.raises(ValueError, match="OPENAI_API_KEY"):
                CodexChallenger(api_key="")

    def test_explicit_key(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            CodexChallenger(api_key="test-key-123")
            mock_openai.OpenAI.assert_called_once_with(api_key="test-key-123")

    def test_env_key(self) -> None:
        with (
            patch.dict("os.environ", {"OPENAI_API_KEY": "env-key-456"}),
            patch("newchan.codex.modes.openai") as mock_openai,
        ):
            CodexChallenger()
            mock_openai.OpenAI.assert_called_once_with(api_key="env-key-456")


class TestReview:
    """review() 方法。"""

    def test_returns_review_result(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_mock_response("无否定。代码逻辑自洽。")
            )

            c = CodexChallenger(api_key="test")
            result = c.review("中枢实现代码", context="src/newchan/core/zhongshu.py")

            assert isinstance(result, ReviewResult)
            assert result.mode == "review"
            assert result.subject == "中枢实现代码"
            assert result.response == "无否定。代码逻辑自洽。"
            assert result.model == "codex-5.3"
            assert "中枢实现代码" in result.prompt
            assert "src/newchan/core/zhongshu.py" in result.prompt

    def test_calls_api_with_correct_model(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_mock_response("ok")
            )

            c = CodexChallenger(api_key="test", model="codex-custom")
            result = c.review("test subject")

            call_args = mock_openai.OpenAI.return_value.responses.create.call_args
            assert call_args.kwargs["model"] == "codex-custom"
            assert result.model == "codex-custom"

    def test_empty_response(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_empty_response()
            )

            c = CodexChallenger(api_key="test")
            result = c.review("test")
            assert result.response == ""

    def test_reasoning_effort_high(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_mock_response("result")
            )

            c = CodexChallenger(api_key="test")
            c.review("test")

            call_args = mock_openai.OpenAI.return_value.responses.create.call_args
            assert call_args.kwargs["reasoning"] == {"effort": "high"}


class TestDiagnose:
    """diagnose() 方法。"""

    def test_returns_diagnose_result(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_mock_response("根因：边界条件缺失。修复：添加空值检查。")
            )

            c = CodexChallenger(api_key="test")
            result = c.diagnose(
                "test_bi_merge FAILED",
                context="AssertionError: expected 3, got 2",
            )

            assert isinstance(result, ReviewResult)
            assert result.mode == "diagnose"
            assert result.subject == "test_bi_merge FAILED"
            assert "根因" in result.response

    def test_empty_response(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_empty_response()
            )

            c = CodexChallenger(api_key="test")
            result = c.diagnose("test")
            assert result.response == ""


class TestDecide:
    """decide() 方法。"""

    def test_returns_decide_result(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_mock_response("决策：使用 frozenset。推理：不可变性优先。")
            )

            c = CodexChallenger(api_key="test")
            result = c.decide(
                "中枢内部线段集合用 list 还是 frozenset",
                context="当前使用 list，但不需要有序访问",
            )

            assert isinstance(result, ReviewResult)
            assert result.mode == "decide"
            assert result.subject == "中枢内部线段集合用 list 还是 frozenset"
            assert result.model == "codex-5.3"


class TestFallback:
    """主模型 5xx 时降级到 fallback。"""

    def test_fallback_on_api_error(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            # 让 mock 的 APIError 指向真实异常类
            mock_openai.APIError = _real_openai.APIError
            mock_client = mock_openai.OpenAI.return_value

            mock_error = _real_openai.APIError(
                message="unavailable",
                request=MagicMock(),
                body=None,
            )
            mock_ok = _make_mock_response("fallback response")
            mock_client.responses.create.side_effect = [
                mock_error,
                mock_ok,
            ]

            c = CodexChallenger(api_key="test")
            result = c.review("test subject")

            assert result.model == "gpt-5.2-codex"
            assert result.response == "fallback response"
            assert mock_client.responses.create.call_count == 2

    def test_no_fallback_when_primary_works(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.OpenAI.return_value.responses.create.return_value = (
                _make_mock_response("primary ok")
            )

            c = CodexChallenger(api_key="test")
            result = c.review("test")

            assert result.model == "codex-5.3"
            assert result.response == "primary ok"

    def test_both_fail_raises(self) -> None:
        with patch("newchan.codex.modes.openai") as mock_openai:
            mock_openai.APIError = _real_openai.APIError
            mock_error = _real_openai.APIError(
                message="unavailable",
                request=MagicMock(),
                body=None,
            )
            mock_openai.OpenAI.return_value.responses.create.side_effect = [
                mock_error,
                mock_error,
            ]

            c = CodexChallenger(api_key="test")
            with pytest.raises(_real_openai.APIError):
                c.review("test")


class TestModuleLevelFunctions:
    """模块级 review()/diagnose()/decide() 便捷函数。"""

    def test_review_auto_init(self) -> None:
        import newchan.codex.modes as mod

        mod._default_challenger = None

        with patch.object(mod, "CodexChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.review.return_value = ReviewResult(
                mode="review", subject="x", response="y", model="m",
            )
            MockClass.return_value = mock_instance

            result = mod.review("x")
            MockClass.assert_called_once()
            mock_instance.review.assert_called_once_with("x", "")
            assert result.response == "y"

        mod._default_challenger = None

    def test_diagnose_auto_init(self) -> None:
        import newchan.codex.modes as mod

        mod._default_challenger = None

        with patch.object(mod, "CodexChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.diagnose.return_value = ReviewResult(
                mode="diagnose", subject="a", response="b", model="m",
            )
            MockClass.return_value = mock_instance

            result = mod.diagnose("a", "ctx")
            mock_instance.diagnose.assert_called_once_with("a", "ctx")
            assert result.response == "b"

        mod._default_challenger = None

    def test_decide_auto_init(self) -> None:
        import newchan.codex.modes as mod

        mod._default_challenger = None

        with patch.object(mod, "CodexChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.decide.return_value = ReviewResult(
                mode="decide", subject="q", response="r", model="m",
            )
            MockClass.return_value = mock_instance

            result = mod.decide("q", "ctx")
            mock_instance.decide.assert_called_once_with("q", "ctx")
            assert result.mode == "decide"
            assert result.response == "r"

        mod._default_challenger = None


class TestCLI:
    """CLI 入口参数解析。"""

    def test_build_parser_review(self) -> None:
        from newchan.codex.__main__ import _build_parser

        parser = _build_parser()
        args = parser.parse_args(["review", "test subject"])
        assert args.mode == "review"
        assert args.subject == "test subject"
        assert args.context == ""
        assert args.context_file is None

    def test_build_parser_diagnose_with_context(self) -> None:
        from newchan.codex.__main__ import _build_parser

        parser = _build_parser()
        args = parser.parse_args([
            "diagnose", "test failure",
            "--context", "error details",
        ])
        assert args.mode == "diagnose"
        assert args.subject == "test failure"
        assert args.context == "error details"

    def test_build_parser_decide(self) -> None:
        from newchan.codex.__main__ import _build_parser

        parser = _build_parser()
        args = parser.parse_args(["decide", "which data structure"])
        assert args.mode == "decide"

    def test_build_parser_context_file(self) -> None:
        from newchan.codex.__main__ import _build_parser

        parser = _build_parser()
        args = parser.parse_args([
            "review", "subject",
            "--context-file", "/tmp/ctx.md",
        ])
        assert args.context_file == "/tmp/ctx.md"


class TestBackwardsCompatShim:
    """向后兼容垫片 codex_challenger.py。"""

    def test_shim_exports(self) -> None:
        import newchan.codex_challenger as shim

        assert hasattr(shim, "CodexChallenger")
        assert hasattr(shim, "ReviewResult")
        assert hasattr(shim, "review")
        assert hasattr(shim, "diagnose")
        assert hasattr(shim, "decide")

    def test_shim_review_function(self) -> None:
        import newchan.codex_challenger as shim

        shim._default_challenger = None

        with patch.object(shim, "CodexChallenger") as MockClass:
            mock_instance = MagicMock()
            mock_instance.review.return_value = ReviewResult(
                mode="review", subject="x", response="y", model="m",
            )
            MockClass.return_value = mock_instance

            result = shim.review("x")
            assert result.response == "y"

        shim._default_challenger = None


class TestReviewResultToMarkdown:
    """ReviewResult.to_markdown() 格式验证。"""

    def test_basic_format(self) -> None:
        ts = datetime(2026, 2, 23, 12, 30, 0, tzinfo=timezone.utc)
        result = ReviewResult(
            mode="review",
            subject="中枢实现代码",
            response="无否定。代码逻辑自洽。",
            model="codex-5.3",
            prompt="## 审查目标\n\n中枢实现代码",
        )
        md = result.to_markdown(ts)

        assert "# Codex review — 2026-02-23 12:30:00 UTC" in md
        assert "- **mode**: review" in md
        assert "- **subject**: 中枢实现代码" in md
        assert "- **model**: codex-5.3" in md
        assert "- **timestamp**: 2026-02-23 12:30:00 UTC" in md
        assert "## Prompt" in md
        assert "## 审查目标" in md
        assert "## Response" in md
        assert "无否定。代码逻辑自洽。" in md

    def test_context_file_included(self) -> None:
        ts = datetime(2026, 2, 23, 12, 30, 0, tzinfo=timezone.utc)
        result = ReviewResult(
            mode="diagnose",
            subject="test failure",
            response="根因：边界缺失",
            model="codex-5.3",
            context_file="/tmp/ctx.md",
        )
        md = result.to_markdown(ts)

        assert "- **context-file**: /tmp/ctx.md" in md

    def test_no_context_file_omitted(self) -> None:
        ts = datetime(2026, 2, 23, 12, 30, 0, tzinfo=timezone.utc)
        result = ReviewResult(
            mode="decide",
            subject="data structure",
            response="use frozenset",
            model="codex-5.3",
        )
        md = result.to_markdown(ts)

        assert "context-file" not in md

    def test_empty_prompt_omitted(self) -> None:
        ts = datetime(2026, 2, 23, 12, 30, 0, tzinfo=timezone.utc)
        result = ReviewResult(
            mode="review",
            subject="test",
            response="ok",
            model="codex-5.3",
            prompt="",
        )
        md = result.to_markdown(ts)

        assert "## Prompt" not in md
        assert "## Response" in md

    def test_default_timestamp(self) -> None:
        result = ReviewResult(
            mode="review", subject="x", response="y", model="m",
        )
        md = result.to_markdown()

        assert "# Codex review —" in md
        assert "UTC" in md

    def test_immutability_preserved(self) -> None:
        """to_markdown 不修改 ReviewResult 对象。"""
        result = ReviewResult(
            mode="review", subject="x", response="y", model="m",
        )
        md1 = result.to_markdown()
        md2 = result.to_markdown()

        assert md1 is not md2
        assert result.mode == "review"
        assert result.subject == "x"


class TestCLISavesResult:
    """CLI 执行后自动持久化到 .chanlun/review-results/。"""

    def test_saves_review_result(self, tmp_path: Path) -> None:
        from newchan.codex.__main__ import _save_result

        result = ReviewResult(
            mode="review",
            subject="test subject",
            response="审查通过",
            model="codex-5.3",
        )
        ts = datetime(2026, 2, 23, 14, 5, 0, tzinfo=timezone.utc)

        with patch("newchan.codex.__main__._RESULTS_DIR", tmp_path):
            saved = _save_result(result, ts)

        assert saved.name == "codex-review-20260223-1405.md"
        assert saved.exists()

        content = saved.read_text(encoding="utf-8")
        assert "# Codex review — 2026-02-23 14:05:00 UTC" in content
        assert "审查通过" in content

    def test_saves_diagnose_result(self, tmp_path: Path) -> None:
        from newchan.codex.__main__ import _save_result

        result = ReviewResult(
            mode="diagnose",
            subject="failure",
            response="根因：X",
            model="codex-5.3",
            context_file="/tmp/ctx.md",
        )
        ts = datetime(2026, 1, 15, 9, 30, 0, tzinfo=timezone.utc)

        with patch("newchan.codex.__main__._RESULTS_DIR", tmp_path):
            saved = _save_result(result, ts)

        assert saved.name == "codex-diagnose-20260115-0930.md"
        content = saved.read_text(encoding="utf-8")
        assert "- **context-file**: /tmp/ctx.md" in content

    def test_main_persists_result(self, tmp_path: Path) -> None:
        """main() 完整调用链验证——结果文件被写入。"""
        with (
            patch("newchan.codex.__main__._RESULTS_DIR", tmp_path),
            patch("newchan.codex.__main__.CodexChallenger") as MockClass,
            patch("newchan.codex.__main__.load_dotenv"),
            patch(
                "sys.argv",
                ["codex", "review", "test subject", "--context", "ctx"],
            ),
        ):
            mock_instance = MagicMock()
            mock_instance.review.return_value = ReviewResult(
                mode="review",
                subject="test subject",
                response="ok",
                model="codex-5.3",
            )
            MockClass.return_value = mock_instance

            from newchan.codex.__main__ import main

            main()

        files = list(tmp_path.glob("codex-review-*.md"))
        assert len(files) == 1
        content = files[0].read_text(encoding="utf-8")
        assert "test subject" in content
        assert "ok" in content

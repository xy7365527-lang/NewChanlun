"""codex_challenger 单元测试（mock codex CLI，不真调 subprocess）

调用链已从 OpenAI API 迁移到 codex CLI（ChatGPT 订阅认证，配额独立）。
测试在两层 mock：
- modes 层：patch engine.call_codex_cli 直接返回 (text, model)
- engine 层：patch subprocess.run 验证 CLI 参数组装与 --output-last-message 解析

覆盖：
1. CodexChallenger 构造（无需 key）
2. review/diagnose/decide 调用链和返回结构
3. engine.call_codex_cli 的 subprocess 参数、输出文件解析、失败路径
4. CLI 参数解析
5. 模块级便捷函数
6. ReviewResult.to_markdown() 格式
7. CLI 持久化文件写入
"""

from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from newchan.codex.modes import (
    CodexChallenger,
    ReviewResult,
)


class TestCodexChallengerInit:
    """构造行为——CLI 版无需 API Key。"""

    def test_no_key_needed(self) -> None:
        # CLI 走 ChatGPT 订阅认证，构造不应报错、不读环境变量。
        c = CodexChallenger()
        assert c._model == "codex-cli"

    def test_custom_model_tag(self) -> None:
        c = CodexChallenger(model="codex-custom")
        assert c._model == "codex-custom"


class TestReview:
    """review() 方法。"""

    def test_returns_review_result(self) -> None:
        with patch("newchan.codex.modes.call_codex_cli") as mock_cli:
            mock_cli.return_value = ("无否定。代码逻辑自洽。", "codex-cli")

            c = CodexChallenger()
            result = c.review("中枢实现代码", context="src/newchan/core/zhongshu.py")

            assert isinstance(result, ReviewResult)
            assert result.mode == "review"
            assert result.subject == "中枢实现代码"
            assert result.response == "无否定。代码逻辑自洽。"
            assert result.model == "codex-cli"
            assert "中枢实现代码" in result.prompt
            assert "src/newchan/core/zhongshu.py" in result.prompt

    def test_passes_system_and_prompt(self) -> None:
        with patch("newchan.codex.modes.call_codex_cli") as mock_cli:
            mock_cli.return_value = ("ok", "codex-custom")

            c = CodexChallenger(model="codex-custom")
            result = c.review("test subject")

            # engine 接收 (prompt, system_prompt, model=...)
            call = mock_cli.call_args
            assert "test subject" in call.args[0]
            assert call.kwargs["model"] == "codex-custom"
            assert result.model == "codex-custom"

    def test_empty_response(self) -> None:
        with patch("newchan.codex.modes.call_codex_cli") as mock_cli:
            mock_cli.return_value = ("", "codex-cli")

            c = CodexChallenger()
            result = c.review("test")
            assert result.response == ""


class TestDiagnose:
    """diagnose() 方法。"""

    def test_returns_diagnose_result(self) -> None:
        with patch("newchan.codex.modes.call_codex_cli") as mock_cli:
            mock_cli.return_value = (
                "根因：边界条件缺失。修复：添加空值检查。", "codex-cli",
            )

            c = CodexChallenger()
            result = c.diagnose(
                "test_bi_merge FAILED",
                context="AssertionError: expected 3, got 2",
            )

            assert isinstance(result, ReviewResult)
            assert result.mode == "diagnose"
            assert result.subject == "test_bi_merge FAILED"
            assert "根因" in result.response


class TestDecide:
    """decide() 方法。"""

    def test_returns_decide_result(self) -> None:
        with patch("newchan.codex.modes.call_codex_cli") as mock_cli:
            mock_cli.return_value = (
                "决策：使用 frozenset。推理：不可变性优先。", "codex-cli",
            )

            c = CodexChallenger()
            result = c.decide(
                "中枢内部线段集合用 list 还是 frozenset",
                context="当前使用 list，但不需要有序访问",
            )

            assert isinstance(result, ReviewResult)
            assert result.mode == "decide"
            assert result.model == "codex-cli"


class TestEngineCLI:
    """engine.call_codex_cli 的 subprocess 组装与解析。"""

    def _mock_run_writing(self, text: str, returncode: int = 0):
        """返回一个 side_effect：把 text 写入 --output-last-message 文件。"""

        def _side_effect(cmd, **kwargs):
            # cmd 里 --output-last-message 的下一个元素是文件路径
            idx = cmd.index("--output-last-message")
            out_path = Path(cmd[idx + 1])
            out_path.write_text(text, encoding="utf-8")
            proc = MagicMock()
            proc.returncode = returncode
            proc.stdout = ""
            proc.stderr = ""
            return proc

        return _side_effect

    def test_cli_args_and_parse(self) -> None:
        from newchan.codex import engine

        captured: dict[str, object] = {}

        def _side_effect(cmd, **kwargs):
            captured["cmd"] = cmd
            captured["cwd"] = kwargs.get("cwd")
            idx = cmd.index("--output-last-message")
            Path(cmd[idx + 1]).write_text("HELLO", encoding="utf-8")
            proc = MagicMock()
            proc.returncode = 0
            return proc

        with patch("newchan.codex.engine.subprocess.run", side_effect=_side_effect):
            text, model = engine.call_codex_cli("PROMPT", "SYS", model="tag")

        assert text == "HELLO"
        assert model == "tag"
        cmd = captured["cmd"]
        assert cmd[:2] == ["codex", "exec"]
        assert "--skip-git-repo-check" in cmd
        assert "--sandbox" in cmd and "read-only" in cmd
        assert "--ephemeral" in cmd
        assert "--output-last-message" in cmd
        # system_prompt 合并进 prompt（最后一个位置参数）
        assert cmd[-1] == "SYS\n\nPROMPT"
        # cwd 是临时目录（非 None）
        assert captured["cwd"] is not None

    def test_no_system_prompt(self) -> None:
        from newchan.codex import engine

        with patch(
            "newchan.codex.engine.subprocess.run",
            side_effect=self._mock_run_writing("R"),
        ) as mock_run:
            text, _ = engine.call_codex_cli("ONLY_PROMPT", "")

        assert text == "R"
        assert mock_run.call_args.args[0][-1] == "ONLY_PROMPT"

    def test_nonzero_exit_raises(self) -> None:
        from newchan.codex import engine

        def _side_effect(cmd, **kwargs):
            proc = MagicMock()
            proc.returncode = 1
            proc.stderr = "boom"
            proc.stdout = ""
            return proc

        with patch("newchan.codex.engine.subprocess.run", side_effect=_side_effect):
            with pytest.raises(RuntimeError, match="codex CLI 失败"):
                engine.call_codex_cli("p", "s")

    def test_missing_output_file_raises(self) -> None:
        from newchan.codex import engine

        def _side_effect(cmd, **kwargs):
            proc = MagicMock()
            proc.returncode = 0  # 成功但没写文件
            return proc

        with patch("newchan.codex.engine.subprocess.run", side_effect=_side_effect):
            with pytest.raises(RuntimeError, match="未写入"):
                engine.call_codex_cli("p", "s")

    def test_timeout_raises(self) -> None:
        import subprocess

        from newchan.codex import engine

        with patch(
            "newchan.codex.engine.subprocess.run",
            side_effect=subprocess.TimeoutExpired(cmd="codex", timeout=1),
        ):
            with pytest.raises(RuntimeError, match="超时"):
                engine.call_codex_cli("p", "s")


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
            model="codex-cli",
            prompt="## 审查目标\n\n中枢实现代码",
        )
        md = result.to_markdown(ts)

        assert "# Codex review — 2026-02-23 12:30:00 UTC" in md
        assert "- **mode**: review" in md
        assert "- **subject**: 中枢实现代码" in md
        assert "- **model**: codex-cli" in md
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
            model="codex-cli",
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
            model="codex-cli",
        )
        md = result.to_markdown(ts)

        assert "context-file" not in md

    def test_empty_prompt_omitted(self) -> None:
        ts = datetime(2026, 2, 23, 12, 30, 0, tzinfo=timezone.utc)
        result = ReviewResult(
            mode="review",
            subject="test",
            response="ok",
            model="codex-cli",
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
            model="codex-cli",
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
            model="codex-cli",
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
                model="codex-cli",
            )
            MockClass.return_value = mock_instance

            from newchan.codex.__main__ import main

            main()

        files = list(tmp_path.glob("codex-review-*.md"))
        assert len(files) == 1
        content = files[0].read_text(encoding="utf-8")
        assert "test subject" in content
        assert "ok" in content

#!/usr/bin/env python
"""Escalate 发送器——通过 Playwright 连接 Chrome 并发送 escalate 包到 claude.ai。

用法:
  python scripts/escalate_send.py --message "escalate 包文本"
  python scripts/escalate_send.py --message "..." --url "https://claude.ai/chat/xxx"
  python scripts/escalate_send.py --message "..." --timeout 300000
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

# Playwright 延迟导入——仅在实际使用时加载
_playwright_available = True
try:
    from playwright.sync_api import sync_playwright, TimeoutError as PwTimeout
except ImportError:
    _playwright_available = False

# ---------------------------------------------------------------------------
# 配置
# ---------------------------------------------------------------------------

DEFAULT_CHROME_USER_DATA = (
    r"C:\Users\hanju\AppData\Local\Google\Chrome\User Data"
)

DEFAULT_TIMEOUT_MS = 300_000  # 5 分钟

# 可能的输入框选择器（按优先级尝试）
INPUT_SELECTORS: tuple[str, ...] = (
    "div.ProseMirror[contenteditable='true']",
    "div[contenteditable='true']",
    "textarea",
)

# 发送按钮选择器
SEND_BUTTON_SELECTORS: tuple[str, ...] = (
    "button[aria-label='Send Message']",
    "button[aria-label='Send message']",
    "button[data-testid='send-button']",
    "button:has(svg):near(div[contenteditable])",
)

# 流式响应检测选择器
STREAMING_INDICATORS: tuple[str, ...] = (
    "button[aria-label='Stop Response']",
    "button[aria-label='Stop generating']",
    "button[data-testid='stop-button']",
)

# 助手消息选择器
ASSISTANT_MESSAGE_SELECTORS: tuple[str, ...] = (
    "[data-testid='assistant-message']:last-child",
    "div[data-is-streaming] >> ..",
    ".font-claude-message:last-child",
)

ERROR_SCREENSHOT_PATH = Path("tmp/escalate_error.png")


# ---------------------------------------------------------------------------
# 配置读取
# ---------------------------------------------------------------------------

def _load_conversation_url(url_override: str = "") -> str:
    """加载 target conversation URL。

    优先级：
    1. CLI 参数 url_override
    2. 环境变量 ESCALATE_CONVERSATION_URL
    3. OpenClaw 配置文件
    """
    if url_override and url_override.strip():
        return url_override.strip()

    env_url = os.environ.get("ESCALATE_CONVERSATION_URL", "").strip()
    if env_url:
        return env_url

    # 尝试从 OpenClaw 配置读取
    openclaw_config_paths = [
        Path.home() / ".openclaw" / "config.json",
        Path.home() / ".openclaw" / "openclaw.json",
        Path(__file__).resolve().parent.parent
        / "topological-computation"
        / "deploy"
        / "openclaw-config.json",
    ]

    for config_path in openclaw_config_paths:
        if config_path.is_file():
            try:
                data = json.loads(config_path.read_text(encoding="utf-8"))
                conv_url = (
                    data.get("target_conversation", "")
                    or data.get("conversation_url", "")
                    or data.get("escalate", {}).get("conversation_url", "")
                )
                if conv_url:
                    return conv_url.strip()
            except (json.JSONDecodeError, KeyError, TypeError):
                continue

    raise ValueError(
        "无法获取 conversation URL。请通过以下方式之一提供：\n"
        "  1. --url 参数\n"
        "  2. ESCALATE_CONVERSATION_URL 环境变量\n"
        "  3. OpenClaw 配置文件中的 target_conversation 字段"
    )


def _load_chrome_user_data(path_override: str = "") -> str:
    """加载 Chrome user data 路径。"""
    if path_override and path_override.strip():
        return path_override.strip()

    env_path = os.environ.get("CHROME_USER_DATA_DIR", "").strip()
    if env_path:
        return env_path

    return DEFAULT_CHROME_USER_DATA


# ---------------------------------------------------------------------------
# Playwright 操作
# ---------------------------------------------------------------------------

def _find_input_element(page):
    """在页面中定位输入框。按选择器优先级依次尝试。"""
    for selector in INPUT_SELECTORS:
        try:
            element = page.wait_for_selector(selector, timeout=3000)
            if element and element.is_visible():
                return element, selector
        except PwTimeout:
            continue

    raise RuntimeError(
        f"无法定位输入框。尝试的选择器: {INPUT_SELECTORS}\n"
        "claude.ai 界面可能已更新，请检查选择器。"
    )


def _send_message(page, input_element, selector: str) -> None:
    """发送消息——先尝试 Enter 键，再尝试点击发送按钮。"""
    # 对 contenteditable div 使用 Enter
    if "contenteditable" in selector:
        input_element.press("Enter")
        return

    # 对 textarea 先试 Enter
    input_element.press("Enter")
    time.sleep(0.5)

    # 检查是否触发了发送（是否出现 streaming indicator）
    for indicator in STREAMING_INDICATORS:
        try:
            page.wait_for_selector(indicator, timeout=2000)
            return  # Enter 键成功
        except PwTimeout:
            continue

    # Enter 没生效，尝试点击发送按钮
    for btn_selector in SEND_BUTTON_SELECTORS:
        try:
            btn = page.wait_for_selector(btn_selector, timeout=2000)
            if btn and btn.is_visible():
                btn.click()
                return
        except PwTimeout:
            continue

    raise RuntimeError(
        "无法发送消息——Enter 键和发送按钮均未生效。"
    )


def _wait_for_response_complete(page, timeout_ms: int) -> None:
    """等待 assistant 响应完成。

    策略：
    1. 先等待 streaming indicator 出现（确认开始生成）
    2. 等待 streaming indicator 消失（确认生成完成）
    """
    # 等待开始生成（最多 30 秒）
    streaming_started = False
    for indicator in STREAMING_INDICATORS:
        try:
            page.wait_for_selector(indicator, timeout=30000)
            streaming_started = True
            break
        except PwTimeout:
            continue

    if not streaming_started:
        # 可能响应太快已经完成，给 3 秒缓冲
        time.sleep(3)
        return

    # 等待生成完成（streaming indicator 消失）
    for indicator in STREAMING_INDICATORS:
        try:
            page.wait_for_selector(
                indicator,
                state="detached",
                timeout=timeout_ms,
            )
            # 给渲染一点时间
            time.sleep(1)
            return
        except PwTimeout:
            continue

    raise TimeoutError(
        f"等待响应超时 ({timeout_ms}ms)。"
        "模型可能仍在生成，或 streaming indicator 选择器已变更。"
    )


def _extract_assistant_response(page) -> str:
    """提取最新的 assistant 消息文本。"""
    for selector in ASSISTANT_MESSAGE_SELECTORS:
        try:
            elements = page.query_selector_all(selector)
            if elements:
                last = elements[-1]
                text = last.inner_text()
                if text and text.strip():
                    return text.strip()
        except Exception:
            continue

    # 兜底：尝试获取最后一个长文本块
    try:
        all_messages = page.query_selector_all("[data-testid*='message']")
        if all_messages:
            last = all_messages[-1]
            return last.inner_text().strip()
    except Exception:
        pass

    return "[无法提取响应文本——请手动检查 claude.ai 页面]"

def send_to_conversation(
    *,
    message: str,
    conversation_url: str,
    chrome_user_data: str,
    timeout_ms: int = DEFAULT_TIMEOUT_MS,
) -> str:
    """通过 Playwright 发送消息到 claude.ai 对话并获取响应。

    返回 assistant 的响应文本。
    """
    if not _playwright_available:
        raise ImportError(
            "playwright 未安装。请运行:\n"
            "  pip install playwright\n"
            "  playwright install chromium"
        )

    with sync_playwright() as pw:
        # 使用 persistent context 连接已登录的 Chrome profile
        context = pw.chromium.launch_persistent_context(
            user_data_dir=chrome_user_data,
            headless=False,
            channel="chrome",
            args=[
                "--disable-blink-features=AutomationControlled",
            ],
            viewport={"width": 1280, "height": 900},
        )

        try:
            page = context.new_page()
            page.goto(conversation_url, wait_until="networkidle")

            # 等待页面加载完成
            page.wait_for_load_state("domcontentloaded")
            time.sleep(2)  # 等待 JS 渲染

            # 定位输入框
            input_element, selector = _find_input_element(page)

            # 输入消息
            input_element.click()
            time.sleep(0.3)

            # 使用 fill 或 keyboard 输入
            if "contenteditable" in selector:
                # contenteditable div——用 keyboard 逐步输入可能太慢
                # 用 JS 直接设置内容
                page.evaluate(
                    """(args) => {
                        const [sel, text] = args;
                        const el = document.querySelector(sel);
                        if (el) {
                            el.focus();
                            el.innerHTML = text.replace(/\\n/g, '<br>');
                            el.dispatchEvent(new Event('input', {bubbles: true}));
                        }
                    }""",
                    [selector, message],
                )
            else:
                input_element.fill(message)

            time.sleep(0.5)

            # 发送
            _send_message(page, input_element, selector)

            # 等待响应完成
            _wait_for_response_complete(page, timeout_ms)

            # 提取响应
            response = _extract_assistant_response(page)
            return response

        except Exception as exc:
            # 出错时截图
            try:
                ERROR_SCREENSHOT_PATH.parent.mkdir(parents=True, exist_ok=True)
                page.screenshot(path=str(ERROR_SCREENSHOT_PATH))
                print(
                    f"错误截图已保存: {ERROR_SCREENSHOT_PATH}",
                    file=sys.stderr,
                )
            except Exception:
                pass
            raise RuntimeError(
                f"Playwright 操作失败: {exc}"
            ) from exc

        finally:
            context.close()


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="通过 Playwright 发送 escalate 包到 claude.ai",
        prog="escalate_send.py",
    )
    parser.add_argument(
        "--message",
        required=True,
        help="要发送的 escalate 包文本",
    )
    parser.add_argument(
        "--url",
        default="",
        help="目标 conversation URL（覆盖配置文件）",
    )
    parser.add_argument(
        "--chrome-profile",
        default="",
        help="Chrome user data 目录路径",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=DEFAULT_TIMEOUT_MS,
        help=f"等待响应超时（毫秒，默认 {DEFAULT_TIMEOUT_MS}）",
    )
    return parser


def main() -> None:
    parser = _build_parser()
    args = parser.parse_args()

    conversation_url = _load_conversation_url(args.url)
    chrome_user_data = _load_chrome_user_data(args.chrome_profile)

    print(f"目标: {conversation_url}", file=sys.stderr)
    print(f"Chrome profile: {chrome_user_data}", file=sys.stderr)

    response = send_to_conversation(
        message=args.message,
        conversation_url=conversation_url,
        chrome_user_data=chrome_user_data,
        timeout_ms=args.timeout,
    )

    # 响应输出到 stdout（供 escalate_router.py 捕获）
    print(response)


if __name__ == "__main__":
    main()

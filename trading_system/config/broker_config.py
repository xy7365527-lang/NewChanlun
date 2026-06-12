"""broker 连接配置（IBKR paper + Binance testnet，设计 §7）。

凭据纪律：全部走环境变量，绝不硬编码。
单进程单 TradingNode（全局单例约束）——IBKR + Binance 并联在单 node 内。

环境变量：
    IBKR  : IBKR_HOST（默认 127.0.0.1）/ IBKR_PORT（paper TWS=7497, paper Gateway=4002）
    Binance: BINANCE_TESTNET_API_KEY / BINANCE_TESTNET_API_SECRET
"""

from __future__ import annotations

import os


def ibkr_paper_config() -> dict:
    """IBKR 纸交易连接参数（TradingNodeConfig 的 data_clients/exec_clients 片段）。

    TODO(阶段4): 安装 `nautilus_trader[ib]` extra 后实装为
        InteractiveBrokersDataClientConfig / InteractiveBrokersExecClientConfig。
    IBKR 角色（风险 R6）：只做执行+对账，期货行情走 Databento Live。
    """
    return {
        "host": os.environ.get("IBKR_HOST", "127.0.0.1"),
        "port": int(os.environ.get("IBKR_PORT", "7497")),  # paper TWS
        "client_id": int(os.environ.get("IBKR_CLIENT_ID", "1")),
        "account_id": os.environ.get("IBKR_ACCOUNT_ID", ""),  # DU... 纸账户
    }


def binance_testnet_config() -> dict:
    """Binance Futures testnet 连接参数。

    恒仓极性表达（在册判决，hold26 正域 L_max≈1.0–1.3x）：
        futures_leverages={"BTCUSDT": 1}（1x）
        futures_margin_types={"BTCUSDT": "isolated"}（逐仓）
    TODO(阶段4): 实装为 BinanceDataClientConfig / BinanceExecClientConfig
        （account_type=USDT_FUTURE, testnet=True, post_only 路径见 LmtExecutor）。
    """
    api_key = os.environ.get("BINANCE_TESTNET_API_KEY", "")
    api_secret = os.environ.get("BINANCE_TESTNET_API_SECRET", "")
    return {
        "api_key": api_key,
        "api_secret": api_secret,
        "testnet": True,
        "account_type": "USDT_FUTURE",
        "futures_leverages": {"BTCUSDT": 1},
        "futures_margin_types": {"BTCUSDT": "isolated"},
    }


def validate_credentials(cfg: dict, required: list[str]) -> None:
    """启动时校验必需凭据存在（fail fast，不静默跑空配置）。"""
    missing = [k for k in required if not cfg.get(k)]
    if missing:
        raise EnvironmentError(f"缺少必需凭据/配置: {missing}（请设置环境变量）")

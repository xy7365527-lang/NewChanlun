"""broker 连接配置（IBKR paper + Hyperliquid，设计 §7 修订版）。

整体架构：IBKR（期货/美股/期权）+ Hyperliquid（加密永续）+ Databento（历史+期货实时）。

凭据纪律：全部走环境变量，绝不硬编码。
单进程单 TradingNode（全局单例约束）——IBKR + Hyperliquid 并联在单 node 内。

环境变量：
    IBKR        : IBKR_HOST（默认 127.0.0.1）/ IBKR_PORT（paper TWS=7497, Gateway=4002）
    Hyperliquid : HYPERLIQUID_PK / HYPERLIQUID_TESTNET_PK（EVM 私钥，adapter 按
                  environment 自动选取）；可选 HYPERLIQUID_VAULT[_TESTNET] /
                  HYPERLIQUID_ACCOUNT_ADDRESS（agent wallet 模式）
    Databento   : DATABENTO_API_KEY

Hyperliquid 调研结论（adapter 源码逐字核对，nautilus_trader 1.228.0）：
    - **官方 adapter 已存在**（adapters/hyperliquid/，pyo3 Rust 实现）——不需要自建。
    - instrument_id 约定: "BTC-USD-PERP.HYPERLIQUID"（perp）/ "PURR-USDC-SPOT.HYPERLIQUID"
    - 行情: subscribe_bars（WS candle）/ subscribe_trade_ticks / l2 book / quote
    - **交易所侧 K 线最小 1m，无 1s**（bar_type_to_interval 白名单:
      1m/3m/5m/15m/30m/1h/2h/4h/8h/12h/1d/3d/1w/1M，且只接受 EXTERNAL）
      ⟹ 1s 床位 = subscribe_trade_ticks → Nautilus INTERNAL 聚合
      （与原 Binance futures 1s 判决同型；testnet 实测收口仍待 L2）
    - 执行: post_only 支持（HL ALO 单；"post only would match" 拒单被 adapter
      显式处理为 due_post_only）——LMT-only + maker 纪律可在交易所级强制执行
    - testnet: HyperliquidEnvironment.TESTNET（无 KYC，水龙头领测试金）
"""

from __future__ import annotations

import os


def ibkr_paper_config() -> dict:
    """IBKR 纸交易连接参数（期货/美股/期权域）。

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


def hyperliquid_config(testnet: bool = True):
    """Hyperliquid data + exec 客户端配置（加密永续域）。

    返回 (HyperliquidDataClientConfig, HyperliquidExecClientConfig)。
    私钥不经本函数传递——adapter 自行从 HYPERLIQUID_PK / HYPERLIQUID_TESTNET_PK
    环境变量读取（按 environment 选取），代码零接触凭据。

    恒仓极性表达（在册判决，hold26 正域 L_max≈1.0–1.3x）：
        HL 杠杆是 per-position 设置（下单时指定，非账户级预设）；
        1x 逐仓约束在 LeverageGovernor 钳制 + 订单参数层表达，
        不依赖交易所侧预配置（与原 Binance futures_leverages 预设机制不同）。
    """
    from nautilus_trader.adapters.hyperliquid.config import (
        HyperliquidDataClientConfig,
        HyperliquidExecClientConfig,
    )
    from nautilus_trader.core.nautilus_pyo3 import HyperliquidEnvironment

    env = HyperliquidEnvironment.TESTNET if testnet else HyperliquidEnvironment.MAINNET
    data_cfg = HyperliquidDataClientConfig(environment=env)
    exec_cfg = HyperliquidExecClientConfig(environment=env)
    return data_cfg, exec_cfg


def databento_live_config() -> dict:
    """Databento Live 行情配置（期货域实时，CME bundle）。

    TODO(阶段4): 实装为 DatabentoDataClientConfig(api_key=..., ...)
    （adapter 已在 1.228.0 内置，LiveDataClientConfig 子类已确认存在）。
    """
    api_key = os.environ.get("DATABENTO_API_KEY", "")
    return {"api_key": api_key}


def validate_credentials(cfg: dict, required: list[str]) -> None:
    """启动时校验必需凭据存在（fail fast，不静默跑空配置）。"""
    missing = [k for k in required if not cfg.get(k)]
    if missing:
        raise EnvironmentError(f"缺少必需凭据/配置: {missing}（请设置环境变量）")

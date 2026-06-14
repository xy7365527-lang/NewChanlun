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

    L2 验证收口（2026-06-12，hl_verify_nautilus.py mainnet 实测）：
    - 私钥显式从 HYPERLIQUID_PRIVATE_KEY 传入——adapter 默认读 HYPERLIQUID_PK /
      HYPERLIQUID_TESTNET_PK，与本机变量名不匹配，缺省路径会静默拿不到凭据；
    - instrument_provider 必须 load_all=True，否则 cache 无 instrument，
      策略 on_start 拿不到合约定义（默认 load_all=False 是验证中第一个失败点）；
    - 订单附带 Nautilus builder code（硬编码，零费归因）——账户须一次性
      approveBuilderFee('0x0c8d970c462726e014ad36f6c5a63e99db48a8e7', '0%')，
      已对主钱包执行（可撤销）。未批准时所有订单被拒。

    恒仓极性表达（在册判决，hold26 正域 L_max≈1.0–1.3x）：
        HL 杠杆是 per-position 设置（下单时指定，非账户级预设）；
        1x 逐仓约束在 LeverageGovernor 钳制 + 订单参数层表达，
        不依赖交易所侧预配置（与原 Binance futures_leverages 预设机制不同）。
    """
    from nautilus_trader.adapters.hyperliquid.config import (
        HyperliquidDataClientConfig,
        HyperliquidExecClientConfig,
    )
    from nautilus_trader.config import InstrumentProviderConfig
    from nautilus_trader.core.nautilus_pyo3 import HyperliquidEnvironment

    env = HyperliquidEnvironment.TESTNET if testnet else HyperliquidEnvironment.MAINNET
    provider = InstrumentProviderConfig(load_all=True)
    data_cfg = HyperliquidDataClientConfig(environment=env, instrument_provider=provider)
    exec_cfg = HyperliquidExecClientConfig(
        environment=env,
        private_key=os.environ.get("HYPERLIQUID_PRIVATE_KEY"),
        instrument_provider=provider,
    )
    return data_cfg, exec_cfg


def databento_live_config(instrument_ids: list[str] | None = None):
    """Databento Live 行情配置（期货域实时，GLBX.MDP3）。

    返回 DatabentoDataClientConfig。api_key 不经本函数传递——adapter 自行从
    DATABENTO_API_KEY 环境变量读取（与 Hyperliquid 同纪律，代码零接触凭据）。

    口径钉死：use_exchange_as_venue=False ⟹ instrument_id 形如 "ESM6.GLBX"，
    与历史 catalog（databento_catalog.py，loader 默认 GLBX venue）一致。
    注意此参数两处 API 默认值相反（loader=False / live=True），必须显式对齐。

    instrument_ids: 启动时请求 definition 并允许订阅的标的，如 ["ESM6.GLBX"]。
    adapter 要求所有待订阅标的在 client 配置中预先声明。
    """
    from nautilus_trader.adapters.databento import DatabentoDataClientConfig
    from nautilus_trader.model.identifiers import InstrumentId

    if not os.environ.get("DATABENTO_API_KEY"):
        raise EnvironmentError("DATABENTO_API_KEY 缺失（检查仓库根 .env 或 shell 环境）")
    return DatabentoDataClientConfig(
        instrument_ids=[InstrumentId.from_str(i) for i in (instrument_ids or [])],
        use_exchange_as_venue=False,
    )


def validate_credentials(cfg: dict, required: list[str]) -> None:
    """启动时校验必需凭据存在（fail fast，不静默跑空配置）。"""
    missing = [k for k in required if not cfg.get(k)]
    if missing:
        raise EnvironmentError(f"缺少必需凭据/配置: {missing}（请设置环境变量）")

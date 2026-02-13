"""项目配置 — 从 .env 读取环境变量"""

import os
from pathlib import Path

from dotenv import load_dotenv

# 优先加载项目根目录下的 .env 文件
_env_path = Path(__file__).resolve().parents[2] / ".env"
load_dotenv(_env_path)

def env_flag(name: str, default: bool = False) -> bool:
    """读取布尔环境变量（支持 1/true/yes/on）。"""
    v = os.getenv(name)
    if v is None:
        return default
    return v.strip().lower() in {"1", "true", "yes", "y", "on"}

ALPHAVANTAGE_API_KEY: str = os.getenv("ALPHAVANTAGE_API_KEY", "")
DATABENTO_API_KEY: str = os.getenv("DATABENTO_API_KEY", "")
CACHE_DIR: str = os.getenv("CACHE_DIR", ".cache")

# IBKR (TWS / IB Gateway) 连接配置
IB_HOST: str = os.getenv("IB_HOST", "127.0.0.1")
IB_PORT: int = int(os.getenv("IB_PORT", "7497"))
IB_CLIENT_ID: int = int(os.getenv("IB_CLIENT_ID", "1"))

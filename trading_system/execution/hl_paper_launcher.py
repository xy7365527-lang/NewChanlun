"""NautilusTrader 0.62 Hyperliquid paper launcher.

The module is deliberately inert by default: ``--dry-run`` validates configuration
without importing Nautilus or opening a network connection.  Live construction is
kept behind ``build_node`` and never submits an order automatically.
"""
from __future__ import annotations

import argparse
import json
import os
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Mapping

from trading_system.execution.risk_controls import OrderIntent, RiskGate, RiskVerdictAction

_ENV_VARS = {"testnet": "HYPERLIQUID_TESTNET_PK", "mainnet": "HYPERLIQUID_PK"}
_SECRET_RE = re.compile(r"(private.?key|secret|password|token|credential|api.?key)", re.I)


class LauncherConfigError(ValueError):
    """Configuration is unsafe or incomplete for the selected environment."""


@dataclass(frozen=True)
class LauncherConfig:
    environment: str
    private_key: str | None
    dry_run: bool = False
    log_path: Path | None = None

    @classmethod
    def from_env(cls, environ: Mapping[str, str] | None = None, *, dry_run: bool = False) -> "LauncherConfig":
        env = os.environ if environ is None else environ
        environment = env.get("HL_ENVIRONMENT", "testnet").strip().lower()
        if environment not in _ENV_VARS:
            raise LauncherConfigError("HL_ENVIRONMENT must be testnet or mainnet")
        key = env.get(_ENV_VARS[environment])
        if not dry_run and not key:
            raise LauncherConfigError(f"missing credential { _ENV_VARS[environment] } for {environment}")
        return cls(environment, key, dry_run, Path(env["HL_PAPER_LOG"]) if env.get("HL_PAPER_LOG") else None)


def _redact(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {k: ("[REDACTED]" if _SECRET_RE.search(str(k)) and not str(k).endswith("_env") else _redact(v)) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [_redact(v) for v in value]
    if isinstance(value, str) and len(value) > 24:
        return "[REDACTED]" if re.fullmatch(r"[A-Za-z0-9_-]{24,}", value) else value
    return value


class ReportSink:
    def __init__(self, path: Path | None = None, emit: Callable[[str], None] | None = None):
        self.path, self.emit = path, emit or print

    def write(self, event: str, **fields: Any) -> None:
        record = {"event": event, **_redact(fields)}
        line = json.dumps(record, ensure_ascii=False, sort_keys=True)
        if self.path:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            self.path.open("a", encoding="utf-8").write(line + "\n")
        self.emit(line)


class HyperliquidPaperFacade:
    """Small Strategy-facing facade; adapter calls are injected for contract tests."""
    def __init__(self, strategy: Any, sink: ReportSink | None = None, *, risk_gate: RiskGate | None = None,
                 equity: Callable[[], float] | None = None, net_position_notional: Callable[[Any], float] | None = None,
                 now_ns: Callable[[], int] | None = None):
        self.strategy, self.sink = strategy, sink or ReportSink()
        self.risk_gate = risk_gate
        self._equity = equity or (lambda: float(strategy.portfolio.account().balance_total().as_double()))
        self._net_position_notional = net_position_notional or (lambda instrument: 0.0)
        self._now_ns = now_ns or (lambda: 0)

    def submit_limit(self, instrument: Any, quantity: Any, price: Any, *, side: Any, client_order_id: Any = None) -> Any:
        client_id = str(client_order_id or "")
        if self.risk_gate is not None:
            verdict = self.risk_gate.check(
                OrderIntent(side=str(side), qty=float(quantity), price=float(price)),
                now_ns=int(self._now_ns()), equity=float(self._equity()),
                net_position_notional=float(self._net_position_notional(instrument)),
            )
            if not verdict.allowed:
                self.sink.write("rejected", client_order_id=client_id, reason=verdict.reason,
                                risk_action=verdict.action.value)
                if verdict.action is RiskVerdictAction.FLATTEN:
                    self._run_kill_actions()
                return None
        order = self.strategy.submit_order(instrument, quantity, price, side=side, client_order_id=client_order_id)
        self.sink.write("accepted", client_order_id=client_id, order_id=str(getattr(order, "client_order_id", order)))
        return order

    def _run_kill_actions(self) -> None:
        """执行 RiskGate 的急停动作；每个 adapter 方法均可注入，便于离线合同测试。"""
        cancel = getattr(self.strategy, "cancel_all_orders", None)
        if callable(cancel):
            cancel()
        flatten = getattr(self.strategy, "close_all_positions", None)
        if callable(flatten):
            flatten()
        self.sink.write("kill_switch_actions", actions=list(self.risk_gate.required_actions if self.risk_gate else ()))

    def cancel(self, order: Any) -> Any:
        result = self.strategy.cancel_order(order)
        self.sink.write("canceled", order_id=str(getattr(order, "client_order_id", order)))
        return result

    def open_orders(self, instrument: Any = None) -> Any:
        return self.strategy.cache.orders_open(instrument) if instrument is not None else self.strategy.cache.orders_open()

    def net_position(self, instrument: Any) -> Any:
        return self.strategy.cache.position(instrument)

    def on_order_rejected(self, event: Any) -> None:
        self.sink.write("rejected", reason=getattr(event, "reason", str(event)), client_order_id=str(getattr(event, "client_order_id", "")))

    def on_order_filled(self, event: Any) -> None:
        self.sink.write("filled", order_id=str(getattr(event, "order_id", "")), fill_qty=str(getattr(event, "last_qty", "")), price=str(getattr(event, "last_px", "")))


def build_node(config: LauncherConfig) -> Any:
    if config.dry_run:
        raise LauncherConfigError("dry-run must not construct a TradingNode")
    # Import only after environment and credential validation.
    try:
        from nautilus_trader.live.node import TradingNode  # type: ignore
        from nautilus_trader.adapters.hyperliquid.config import HyperliquidDataClientConfig, HyperliquidExecClientConfig  # type: ignore
    except ImportError as exc:
        raise RuntimeError("NautilusTrader 0.62 Hyperliquid adapter is required for live construction") from exc
    # Adapter configuration signatures differ across patch releases; retain the
    # validated key in a narrow config object and let the installed adapter reject
    # unsupported fields before any order is sent.
    node = TradingNode(config=type("NodeConfig", (), {"environment": config.environment, "private_key": config.private_key})())
    return node


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Hyperliquid paper launcher")
    parser.add_argument("--dry-run", action="store_true", help="validate environment without importing Nautilus or networking")
    args = parser.parse_args(argv)
    config = LauncherConfig.from_env(dry_run=args.dry_run)
    ReportSink(config.log_path).write("dry_run" if args.dry_run else "configured", environment=config.environment, credential_env=_ENV_VARS[config.environment])
    if not args.dry_run:
        build_node(config)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

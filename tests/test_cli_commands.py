"""Tests for CLI command handler functions in newchan.cli."""

from __future__ import annotations

import argparse
import sys
import types
from unittest.mock import MagicMock, patch

import pandas as pd
import pytest

# Pre-inject a stub for newchan.data_ibkr so that patch() can resolve
# the dotted path even when ib_insync is not installed.
import newchan as _newchan_pkg

if "newchan.data_ibkr" not in sys.modules:
    _stub_ibkr = types.ModuleType("newchan.data_ibkr")
    _stub_ibkr.IBKRProvider = MagicMock  # type: ignore[attr-defined]
    sys.modules["newchan.data_ibkr"] = _stub_ibkr
    _newchan_pkg.data_ibkr = _stub_ibkr  # type: ignore[attr-defined]

from newchan.cli import (
    _cmd_fetch,
    _cmd_fetch_db,
    _cmd_synthetic,
    _load_cache_or_exit,
    main,
)


# ===================================================================
# Helpers
# ===================================================================


def _make_df(n: int = 5) -> pd.DataFrame:
    """Return a small dummy DataFrame for testing."""
    return pd.DataFrame({"close": range(n)})


# ===================================================================
# _load_cache_or_exit
# ===================================================================


class TestLoadCacheOrExit:
    """Tests for _load_cache_or_exit."""

    @patch("newchan.cache.load_df")
    def test_cache_hit_returns_df(self, mock_load_df: MagicMock) -> None:
        """When cache exists, return the DataFrame."""
        expected = _make_df()
        mock_load_df.return_value = expected

        result = _load_cache_or_exit("CL_1min_raw", "CL", "1min")

        mock_load_df.assert_called_once_with("CL_1min_raw")
        pd.testing.assert_frame_equal(result, expected)

    @patch("newchan.cache.load_df")
    def test_cache_miss_exits(self, mock_load_df: MagicMock, capsys) -> None:
        """When cache is None, print error to stderr and sys.exit(1)."""
        mock_load_df.return_value = None

        with pytest.raises(SystemExit) as exc_info:
            _load_cache_or_exit("CL_1min_raw", "CL", "1min")

        assert exc_info.value.code == 1
        captured = capsys.readouterr()
        assert "CL_1min_raw" in captured.err


# ===================================================================
# _cmd_fetch
# ===================================================================


class TestCmdFetch:
    """Tests for _cmd_fetch."""

    def _base_args(self, **overrides) -> argparse.Namespace:
        defaults = dict(
            symbol="CL",
            source="ibkr",
            interval="1min",
            refresh=False,
            duration="2 D",
        )
        defaults.update(overrides)
        return argparse.Namespace(**defaults)

    # -- cache hit (no refresh) --

    @patch("newchan.cache.load_df")
    def test_cache_hit_skips_fetch(self, mock_load_df: MagicMock, capsys) -> None:
        """When cache hit and no --refresh, print and return early."""
        mock_load_df.return_value = _make_df(10)

        _cmd_fetch(self._base_args())

        mock_load_df.assert_called_once_with("CL_1min_raw")
        captured = capsys.readouterr()
        assert "10" in captured.out

    # -- IBKR source --

    @patch("newchan.cache.save_df")
    @patch("newchan.convert.bars_to_df")
    @patch("newchan.data_ibkr.IBKRProvider")
    @patch("newchan.cache.load_df")
    def test_ibkr_happy_path(
        self,
        mock_load_df: MagicMock,
        mock_ibkr_cls: MagicMock,
        mock_bars_to_df: MagicMock,
        mock_save_df: MagicMock,
        capsys,
    ) -> None:
        """IBKR source: fetch bars, convert, save."""
        mock_load_df.return_value = None  # cache miss

        mock_provider = MagicMock()
        mock_provider.fetch_historical.return_value = [{"bar": 1}]
        mock_ibkr_cls.return_value.__enter__ = MagicMock(return_value=mock_provider)
        mock_ibkr_cls.return_value.__exit__ = MagicMock(return_value=False)

        df = _make_df(3)
        mock_bars_to_df.return_value = df
        mock_save_df.return_value = "/tmp/CL_1min_raw.parquet"

        _cmd_fetch(self._base_args(refresh=True))

        mock_provider.fetch_historical.assert_called_once_with(
            symbol="CL", interval="1min", duration="2 D"
        )
        mock_bars_to_df.assert_called_once_with([{"bar": 1}])
        mock_save_df.assert_called_once_with("CL_1min_raw", df)

    # -- IBKR no bars --

    @patch("newchan.data_ibkr.IBKRProvider")
    @patch("newchan.cache.load_df")
    def test_ibkr_no_bars(
        self, mock_load_df: MagicMock, mock_ibkr_cls: MagicMock, capsys
    ) -> None:
        """When IBKR returns empty bars, print warning and return."""
        mock_load_df.return_value = None

        mock_provider = MagicMock()
        mock_provider.fetch_historical.return_value = []
        mock_ibkr_cls.return_value.__enter__ = MagicMock(return_value=mock_provider)
        mock_ibkr_cls.return_value.__exit__ = MagicMock(return_value=False)

        _cmd_fetch(self._base_args(refresh=True))

        captured = capsys.readouterr()
        assert "警告" in captured.out

    # -- AV source: BRENT --

    @patch("newchan.cache.save_df")
    @patch("newchan.convert.bars_to_df")
    @patch("newchan.data_av.AlphaVantageProvider")
    @patch("newchan.cli._check_av_env")
    @patch("newchan.cache.load_df")
    def test_av_brent_happy_path(
        self,
        mock_load_df: MagicMock,
        mock_check_env: MagicMock,
        mock_av_cls: MagicMock,
        mock_bars_to_df: MagicMock,
        mock_save_df: MagicMock,
        capsys,
    ) -> None:
        """AV source with BRENT symbol fetches brent daily data."""
        mock_load_df.return_value = None

        mock_provider = MagicMock()
        mock_provider.fetch_brent_daily.return_value = [{"bar": "brent"}]
        mock_av_cls.return_value = mock_provider

        df = _make_df(2)
        mock_bars_to_df.return_value = df
        mock_save_df.return_value = "/tmp/BRENT_1min_raw.parquet"

        _cmd_fetch(self._base_args(symbol="brent", source="av", refresh=True))

        mock_check_env.assert_called_once()
        mock_provider.fetch_brent_daily.assert_called_once()
        mock_bars_to_df.assert_called_once_with([{"bar": "brent"}])
        mock_save_df.assert_called_once_with("BRENT_1min_raw", df)

    # -- AV unsupported symbol --

    @patch("newchan.cli._check_av_env")
    @patch("newchan.cache.load_df")
    def test_av_unsupported_symbol_exits(
        self, mock_load_df: MagicMock, mock_check_env: MagicMock, capsys
    ) -> None:
        """AV source with unsupported symbol exits with error."""
        mock_load_df.return_value = None

        with pytest.raises(SystemExit) as exc_info:
            _cmd_fetch(self._base_args(symbol="GOLD", source="av", refresh=True))

        assert exc_info.value.code == 1
        captured = capsys.readouterr()
        assert "GOLD" in captured.err

    # -- Unknown source --

    @patch("newchan.cache.load_df")
    def test_unknown_source_exits(self, mock_load_df: MagicMock, capsys) -> None:
        """Unknown source exits with error."""
        mock_load_df.return_value = None
        args = self._base_args(source="xyz", refresh=True)

        with pytest.raises(SystemExit) as exc_info:
            _cmd_fetch(args)

        assert exc_info.value.code == 1
        captured = capsys.readouterr()
        assert "xyz" in captured.err

    # -- symbol uppercased --

    @patch("newchan.cache.load_df")
    def test_symbol_is_uppercased(self, mock_load_df: MagicMock) -> None:
        """Symbol is uppercased for cache key."""
        mock_load_df.return_value = _make_df()

        _cmd_fetch(self._base_args(symbol="cl"))

        mock_load_df.assert_called_once_with("CL_1min_raw")


# ===================================================================
# _cmd_synthetic
# ===================================================================


class TestCmdSynthetic:
    """Tests for _cmd_synthetic."""

    def _base_args(self, **overrides) -> argparse.Namespace:
        defaults = dict(
            sym_a="CL",
            sym_b="GC",
            interval="1min",
            op="spread",
        )
        defaults.update(overrides)
        return argparse.Namespace(**defaults)

    @patch("newchan.cache.save_df")
    @patch("newchan.synthetic.make_spread")
    @patch("newchan.cache.load_df")
    def test_spread_op(
        self,
        mock_load_df: MagicMock,
        mock_make_spread: MagicMock,
        mock_save_df: MagicMock,
        capsys,
    ) -> None:
        """Spread operation loads both caches and calls make_spread."""
        df_a = _make_df(4)
        df_b = _make_df(4)
        df_synth = _make_df(4)

        mock_load_df.side_effect = [df_a, df_b]
        mock_make_spread.return_value = df_synth
        mock_save_df.return_value = "/tmp/synth.parquet"

        _cmd_synthetic(self._base_args(op="spread"))

        assert mock_load_df.call_count == 2
        mock_load_df.assert_any_call("CL_1min_raw")
        mock_load_df.assert_any_call("GC_1min_raw")
        mock_make_spread.assert_called_once_with(df_a, df_b)
        mock_save_df.assert_called_once_with("CL_GC_spread_1min_raw", df_synth)

    @patch("newchan.cache.save_df")
    @patch("newchan.synthetic.make_ratio")
    @patch("newchan.cache.load_df")
    def test_ratio_op(
        self,
        mock_load_df: MagicMock,
        mock_make_ratio: MagicMock,
        mock_save_df: MagicMock,
        capsys,
    ) -> None:
        """Ratio operation loads both caches and calls make_ratio."""
        df_a = _make_df(3)
        df_b = _make_df(3)
        df_synth = _make_df(3)

        mock_load_df.side_effect = [df_a, df_b]
        mock_make_ratio.return_value = df_synth
        mock_save_df.return_value = "/tmp/synth.parquet"

        _cmd_synthetic(self._base_args(op="ratio"))

        mock_make_ratio.assert_called_once_with(df_a, df_b)
        mock_save_df.assert_called_once_with("CL_GC_ratio_1min_raw", df_synth)

    @patch("newchan.cache.load_df")
    def test_cache_miss_a_exits(self, mock_load_df: MagicMock) -> None:
        """If cache for sym_a is missing, exits with error."""
        mock_load_df.return_value = None

        with pytest.raises(SystemExit) as exc_info:
            _cmd_synthetic(self._base_args())

        assert exc_info.value.code == 1


# ===================================================================
# _cmd_fetch_db
# ===================================================================


class TestCmdFetchDb:
    """Tests for _cmd_fetch_db."""

    def _base_args(self, **overrides) -> argparse.Namespace:
        defaults = dict(
            fetch_all=False,
            symbol=None,
            intervals="1min,1day",
            start="2010-01-01",
        )
        defaults.update(overrides)
        return argparse.Namespace(**defaults)

    @patch("newchan.data_databento.DEFAULT_SYMBOLS", ["CL", "GC"])
    @patch("newchan.data_databento.fetch_and_cache")
    def test_fetch_all_flag(
        self, mock_fetch: MagicMock, capsys
    ) -> None:
        """--all flag uses DEFAULT_SYMBOLS."""
        mock_fetch.return_value = ("CL_1min_raw", 100)

        _cmd_fetch_db(self._base_args(fetch_all=True))

        # 2 symbols x 2 intervals = 4 calls
        assert mock_fetch.call_count == 4
        captured = capsys.readouterr()
        assert "完成" in captured.out

    @patch("newchan.data_databento.fetch_and_cache")
    def test_symbol_arg(self, mock_fetch: MagicMock, capsys) -> None:
        """--symbol splits by comma and uppercases."""
        mock_fetch.return_value = ("ES_1min_raw", 50)

        _cmd_fetch_db(self._base_args(symbol="es,nq"))

        # 2 symbols x 2 intervals = 4 calls
        assert mock_fetch.call_count == 4
        # Check that symbols are uppercased
        called_symbols = [c.args[0] for c in mock_fetch.call_args_list]
        assert "ES" in called_symbols
        assert "NQ" in called_symbols

    def test_no_symbol_no_all_exits(self, capsys) -> None:
        """Neither --symbol nor --all exits with error."""
        with pytest.raises(SystemExit) as exc_info:
            _cmd_fetch_db(self._base_args())

        assert exc_info.value.code == 1
        captured = capsys.readouterr()
        assert "错误" in captured.err

    @patch("newchan.data_databento.fetch_and_cache")
    def test_exception_handling(self, mock_fetch: MagicMock, capsys) -> None:
        """Exceptions in fetch_and_cache are caught per-iteration."""
        mock_fetch.side_effect = [
            ("CL_1min_raw", 100),
            RuntimeError("API error"),
            ("CL_1day_raw", 200),
            Exception("timeout"),
        ]

        _cmd_fetch_db(self._base_args(symbol="CL,GC"))

        # Should complete (not raise) even with errors
        captured = capsys.readouterr()
        assert "失败" in captured.out
        assert "完成" in captured.out

    @patch("newchan.data_databento.fetch_and_cache")
    def test_single_symbol_single_interval(self, mock_fetch: MagicMock, capsys) -> None:
        """Single symbol with single interval makes 1 call."""
        mock_fetch.return_value = ("AMD_1day_raw", 3000)

        _cmd_fetch_db(self._base_args(symbol="AMD", intervals="1day"))

        mock_fetch.assert_called_once_with("AMD", "1day", start="2010-01-01")
        captured = capsys.readouterr()
        assert "3000" in captured.out


# ===================================================================
# main (dispatch)
# ===================================================================


class TestMain:
    """Tests for main() dispatch."""

    @patch("newchan.cli.build_parser")
    def test_no_command_prints_help(self, mock_build_parser: MagicMock) -> None:
        """When args.command is None, print help."""
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = argparse.Namespace(command=None)
        mock_build_parser.return_value = mock_parser

        main()

        mock_parser.print_help.assert_called_once()

    @patch("newchan.cli._cmd_fetch")
    @patch("newchan.cli.build_parser")
    def test_fetch_command_dispatches(
        self, mock_build_parser: MagicMock, mock_cmd_fetch: MagicMock
    ) -> None:
        """'fetch' command dispatches to _cmd_fetch."""
        args = argparse.Namespace(command="fetch")
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = args
        mock_build_parser.return_value = mock_parser

        main()

        mock_cmd_fetch.assert_called_once_with(args)

    @patch("newchan.cli._cmd_synthetic")
    @patch("newchan.cli.build_parser")
    def test_synthetic_command_dispatches(
        self, mock_build_parser: MagicMock, mock_cmd_synthetic: MagicMock
    ) -> None:
        """'synthetic' command dispatches to _cmd_synthetic."""
        args = argparse.Namespace(command="synthetic")
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = args
        mock_build_parser.return_value = mock_parser

        main()

        mock_cmd_synthetic.assert_called_once_with(args)

    @patch("newchan.cli._cmd_fetch_db")
    @patch("newchan.cli.build_parser")
    def test_fetch_db_command_dispatches(
        self, mock_build_parser: MagicMock, mock_cmd_fetch_db: MagicMock
    ) -> None:
        """'fetch-db' command dispatches to _cmd_fetch_db."""
        args = argparse.Namespace(command="fetch-db")
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = args
        mock_build_parser.return_value = mock_parser

        main()

        mock_cmd_fetch_db.assert_called_once_with(args)

    @patch("newchan.cli._cmd_plot")
    @patch("newchan.cli.build_parser")
    def test_plot_command_dispatches(
        self, mock_build_parser: MagicMock, mock_cmd_plot: MagicMock
    ) -> None:
        """'plot' command dispatches to _cmd_plot."""
        args = argparse.Namespace(command="plot")
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = args
        mock_build_parser.return_value = mock_parser

        main()

        mock_cmd_plot.assert_called_once_with(args)

    @patch("newchan.server.run_server")
    @patch("newchan.cli.build_parser")
    def test_chart_command_dispatches(
        self, mock_build_parser: MagicMock, mock_run_server: MagicMock
    ) -> None:
        """'chart' command dispatches to run_server."""
        args = argparse.Namespace(command="chart", port=9999)
        mock_parser = MagicMock()
        mock_parser.parse_args.return_value = args
        mock_build_parser.return_value = mock_parser

        main()

        mock_run_server.assert_called_once_with(port=9999)

"""K4 配置编码器测试。

测试覆盖：
  - 不可变性（frozen dataclass）
  - 默认配置完整性
  - 完全图约束验证
  - 顶点/边一致性验证
  - JSON 序列化/反序列化往返
  - YAML 序列化/反序列化往返
  - 文件 I/O
  - 边界情况（缺顶点、缺边、重复、自环、symbol 不一致）
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from newchan.k4_config import (
    BasisParams,
    EdgeConfig,
    EffDimParams,
    K4Config,
    ValidationThresholds,
    VertexConfig,
    config_from_dict,
    config_from_json,
    config_to_dict,
    config_to_json,
    load_config,
    make_default_config,
    save_config,
    validate_config,
)
from newchan.matrix_topology import ALL_EDGES, AssetVertex


# ── VertexConfig 不可变性 ─────────────────────────────────


class TestVertexConfig:
    def test_frozen(self):
        vc = VertexConfig(
            vertex=AssetVertex.EQUITY,
            symbol="^GSPC",
            display_name="S&P 500",
        )
        with pytest.raises(AttributeError):
            vc.symbol = "SPY"  # type: ignore[misc]

    def test_default_data_source(self):
        vc = VertexConfig(
            vertex=AssetVertex.EQUITY,
            symbol="^GSPC",
            display_name="S&P 500",
        )
        assert vc.data_source == "yfinance"


# ── EdgeConfig 不可变性 ──────────────────────────────────


class TestEdgeConfig:
    def test_frozen(self):
        ec = EdgeConfig(
            vertex_a=AssetVertex.EQUITY,
            vertex_b=AssetVertex.CASH,
            sym_a="^GSPC",
            sym_b="GC=F",
        )
        with pytest.raises(AttributeError):
            ec.sym_a = "SPY"  # type: ignore[misc]


# ── ValidationThresholds 默认值 ──────────────────────────


class TestValidationThresholds:
    def test_defaults(self):
        vt = ValidationThresholds()
        assert vt.t_cv == 0.01
        assert vt.t_stroke_pct == 0.005
        assert vt.t_dynamics_pct == 0.0001
        assert vt.t_liquidity == 0.5
        assert vt.min_overlap == 5

    def test_frozen(self):
        vt = ValidationThresholds()
        with pytest.raises(AttributeError):
            vt.t_cv = 0.02  # type: ignore[misc]


# ── EffDimParams 默认值 ──────────────────────────────────


class TestEffDimParams:
    def test_defaults(self):
        ep = EffDimParams()
        assert ep.window == 252
        assert ep.percentile_threshold == 5.0


# ── BasisParams 默认值 ───────────────────────────────────


class TestBasisParams:
    def test_defaults(self):
        bp = BasisParams()
        assert bp.rolling_window == 20
        assert bp.z_threshold == 2.0
        assert bp.gld_to_oz == 10.0


# ── 默认配置 ─────────────────────────────────────────────


class TestDefaultConfig:
    def test_has_four_vertices(self):
        cfg = make_default_config()
        assert len(cfg.vertices) == 4

    def test_covers_all_vertices(self):
        cfg = make_default_config()
        vertex_set = {v.vertex for v in cfg.vertices}
        assert vertex_set == set(AssetVertex)

    def test_has_six_edges(self):
        cfg = make_default_config()
        assert len(cfg.edges) == 6

    def test_covers_all_edge_combinations(self):
        cfg = make_default_config()
        actual = {frozenset([e.vertex_a, e.vertex_b]) for e in cfg.edges}
        expected = {frozenset(pair) for pair in ALL_EDGES}
        assert actual == expected

    def test_region_default(self):
        cfg = make_default_config()
        assert cfg.region == "US"

    def test_region_custom(self):
        cfg = make_default_config(region="CN")
        assert cfg.region == "CN"

    def test_default_validation_thresholds(self):
        cfg = make_default_config()
        assert cfg.validation.t_cv == 0.01
        assert cfg.validation.min_overlap == 5

    def test_default_effdim_params(self):
        cfg = make_default_config()
        assert cfg.effdim.window == 252

    def test_default_basis_params(self):
        cfg = make_default_config()
        assert cfg.basis.gld_to_oz == 10.0

    def test_validation_passes(self):
        cfg = make_default_config()
        errors = validate_config(cfg)
        assert errors == []

    def test_edge_symbols_match_vertices(self):
        """边的 sym_a/sym_b 与对应顶点的 symbol 一致。"""
        cfg = make_default_config()
        vertex_sym = {v.vertex: v.symbol for v in cfg.vertices}
        for edge in cfg.edges:
            assert edge.sym_a == vertex_sym[edge.vertex_a]
            assert edge.sym_b == vertex_sym[edge.vertex_b]

    def test_edges_have_descriptions(self):
        """默认配置的每条边都有描述。"""
        cfg = make_default_config()
        for edge in cfg.edges:
            assert edge.description != ""

    def test_frozen(self):
        cfg = make_default_config()
        with pytest.raises(AttributeError):
            cfg.region = "EU"  # type: ignore[misc]


# ── 验证 ─────────────────────────────────────────────────


class TestValidation:
    def _make_valid_config(self) -> K4Config:
        return make_default_config()

    def test_missing_vertex(self):
        """缺少顶点 → 报错。"""
        cfg = self._make_valid_config()
        # 去掉一个顶点
        short_vertices = cfg.vertices[:3]
        bad_cfg = K4Config(
            region="US",
            vertices=short_vertices,
            edges=cfg.edges,
        )
        errors = validate_config(bad_cfg)
        assert any("需要恰好 4 个顶点" in e for e in errors)

    def test_missing_edge(self):
        """缺少边 → 报错。"""
        cfg = self._make_valid_config()
        short_edges = cfg.edges[:5]
        bad_cfg = K4Config(
            region="US",
            vertices=cfg.vertices,
            edges=short_edges,
        )
        errors = validate_config(bad_cfg)
        assert any("需要恰好 6 条边" in e for e in errors)

    def test_duplicate_vertex(self):
        """重复顶点 → 报错。"""
        cfg = self._make_valid_config()
        dup_vertices = cfg.vertices[:3] + (cfg.vertices[0],)
        bad_cfg = K4Config(
            region="US",
            vertices=dup_vertices,
            edges=cfg.edges,
        )
        errors = validate_config(bad_cfg)
        assert any("重复顶点" in e or "缺少顶点" in e for e in errors)

    def test_duplicate_edge(self):
        """重复边 → 报错。"""
        cfg = self._make_valid_config()
        dup_edges = cfg.edges[:5] + (cfg.edges[0],)
        bad_cfg = K4Config(
            region="US",
            vertices=cfg.vertices,
            edges=dup_edges,
        )
        errors = validate_config(bad_cfg)
        assert any("重复边" in e or "缺少边" in e for e in errors)

    def test_self_loop_edge(self):
        """自环边 → 报错。"""
        cfg = self._make_valid_config()
        self_loop = EdgeConfig(
            vertex_a=AssetVertex.EQUITY,
            vertex_b=AssetVertex.EQUITY,
            sym_a="^GSPC",
            sym_b="^GSPC",
        )
        bad_edges = cfg.edges[:5] + (self_loop,)
        bad_cfg = K4Config(
            region="US",
            vertices=cfg.vertices,
            edges=bad_edges,
        )
        errors = validate_config(bad_cfg)
        assert any("自环" in e for e in errors)

    def test_symbol_mismatch(self):
        """边的 symbol 与顶点不一致 → 报错。"""
        cfg = self._make_valid_config()
        # 替换第一条边的 sym_a
        first = cfg.edges[0]
        bad_edge = EdgeConfig(
            vertex_a=first.vertex_a,
            vertex_b=first.vertex_b,
            sym_a="WRONG_SYMBOL",
            sym_b=first.sym_b,
            description=first.description,
        )
        bad_edges = (bad_edge,) + cfg.edges[1:]
        bad_cfg = K4Config(
            region="US",
            vertices=cfg.vertices,
            edges=bad_edges,
        )
        errors = validate_config(bad_cfg)
        assert any("不一致" in e for e in errors)


# ── JSON 序列化 ──────────────────────────────────────────


class TestJsonSerialization:
    def test_roundtrip(self):
        """JSON 序列化→反序列化往返不变。"""
        cfg = make_default_config()
        json_str = config_to_json(cfg)
        restored = config_from_json(json_str)
        assert restored.region == cfg.region
        assert len(restored.vertices) == len(cfg.vertices)
        assert len(restored.edges) == len(cfg.edges)
        for orig, rest in zip(cfg.vertices, restored.vertices):
            assert orig.vertex == rest.vertex
            assert orig.symbol == rest.symbol
            assert orig.display_name == rest.display_name
            assert orig.data_source == rest.data_source
        for orig, rest in zip(cfg.edges, restored.edges):
            assert orig.vertex_a == rest.vertex_a
            assert orig.vertex_b == rest.vertex_b
            assert orig.sym_a == rest.sym_a
            assert orig.sym_b == rest.sym_b
            assert orig.description == rest.description

    def test_dict_roundtrip(self):
        cfg = make_default_config()
        d = config_to_dict(cfg)
        restored = config_from_dict(d)
        assert restored.validation.t_cv == cfg.validation.t_cv
        assert restored.effdim.window == cfg.effdim.window
        assert restored.basis.gld_to_oz == cfg.basis.gld_to_oz

    def test_json_is_valid_json(self):
        cfg = make_default_config()
        json_str = config_to_json(cfg)
        parsed = json.loads(json_str)
        assert isinstance(parsed, dict)
        assert "region" in parsed
        assert "vertices" in parsed
        assert "edges" in parsed

    def test_validation_params_roundtrip(self):
        cfg = K4Config(
            region="EU",
            vertices=make_default_config().vertices,
            edges=make_default_config().edges,
            validation=ValidationThresholds(
                t_cv=0.05,
                t_stroke_pct=0.01,
                t_dynamics_pct=0.001,
                t_liquidity=0.3,
                min_overlap=10,
            ),
            effdim=EffDimParams(window=126, percentile_threshold=10.0),
            basis=BasisParams(rolling_window=10, z_threshold=1.5, gld_to_oz=5.0),
        )
        restored = config_from_json(config_to_json(cfg))
        assert restored.validation.t_cv == 0.05
        assert restored.validation.min_overlap == 10
        assert restored.effdim.window == 126
        assert restored.basis.z_threshold == 1.5

    def test_unknown_vertex_raises(self):
        d = config_to_dict(make_default_config())
        d["vertices"][0]["vertex"] = "NONEXISTENT"
        with pytest.raises(ValueError, match="未知顶点"):
            config_from_dict(d)


# ── 文件 I/O ─────────────────────────────────────────────


class TestFileIO:
    def test_save_and_load(self, tmp_path: Path):
        cfg = make_default_config()
        filepath = tmp_path / "k4_config.json"
        save_config(cfg, filepath)
        loaded = load_config(filepath)
        assert loaded.region == cfg.region
        assert len(loaded.vertices) == 4
        assert len(loaded.edges) == 6
        errors = validate_config(loaded)
        assert errors == []

    def test_save_creates_valid_json(self, tmp_path: Path):
        cfg = make_default_config()
        filepath = tmp_path / "k4.json"
        save_config(cfg, filepath)
        content = filepath.read_text(encoding="utf-8")
        parsed = json.loads(content)
        assert parsed["region"] == "US"


# ── YAML 序列化（需要 pyyaml）────────────────────────────


class TestYamlSerialization:
    @pytest.fixture(autouse=True)
    def _check_yaml(self):
        pytest.importorskip("yaml")

    def test_roundtrip(self):
        from newchan.k4_config import config_from_yaml, config_to_yaml

        cfg = make_default_config()
        yaml_str = config_to_yaml(cfg)
        restored = config_from_yaml(yaml_str)
        assert restored.region == cfg.region
        assert len(restored.vertices) == 4
        assert len(restored.edges) == 6

    def test_yaml_is_string(self):
        from newchan.k4_config import config_to_yaml

        cfg = make_default_config()
        yaml_str = config_to_yaml(cfg)
        assert isinstance(yaml_str, str)
        assert "region" in yaml_str

"""Tests for downstream_audit.py verification hints (154号-2 优化)."""
import json
import os
import sys
import tempfile
import textwrap

import pytest
import yaml

# Ensure scripts/ is importable
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "scripts"))

from downstream_audit import (
    audit,
    load_verification_hints,
    verify_by_hint,
    extract_downstream_actions,
    check_action_resolved,
    load_overrides,
)


@pytest.fixture
def tmp_project(tmp_path):
    """Create a minimal project structure for testing."""
    settled_dir = tmp_path / ".chanlun" / "genealogy" / "settled"
    settled_dir.mkdir(parents=True)
    return tmp_path


def _write_genealogy(settled_dir, gid, downstream_actions, negates=None):
    """Helper: write a minimal genealogy file with downstream actions."""
    negates_line = f"negates: [{negates}]" if negates else "negates: []"
    actions_text = "\n".join(
        f"{i}. {text}" for i, text in enumerate(downstream_actions, 1)
    )
    content = textwrap.dedent(f"""\
        ---
        id: "{gid}"
        status: 已结算
        {negates_line}
        ---
        # {gid}号谱系

        ## 下游推论

        {actions_text}

        ## 影响声明

        无
    """)
    (settled_dir / f"{gid}-test.md").write_text(content, encoding="utf-8")


class TestLoadVerificationHints:
    def test_no_file_returns_empty(self, tmp_project):
        assert load_verification_hints(str(tmp_project)) == {}

    def test_valid_hints_loaded(self, tmp_project):
        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text(
            yaml.dump({"hints": {
                "155-1": [{"file": "foo.txt", "pattern": "bar"}],
            }}),
            encoding="utf-8",
        )
        result = load_verification_hints(str(tmp_project))
        assert "155-1" in result
        assert result["155-1"][0]["file"] == "foo.txt"
        assert result["155-1"][0]["pattern"] == "bar"

    def test_malformed_yaml_returns_empty(self, tmp_project):
        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text("{{{{invalid yaml", encoding="utf-8")
        assert load_verification_hints(str(tmp_project)) == {}

    def test_missing_hints_key_returns_empty(self, tmp_project):
        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text(yaml.dump({"other_key": "value"}), encoding="utf-8")
        assert load_verification_hints(str(tmp_project)) == {}


class TestVerifyByHint:
    def test_all_hints_match(self, tmp_project):
        target = tmp_project / "target.txt"
        target.write_text("hello world Codex here", encoding="utf-8")
        hints = [{"file": "target.txt", "pattern": "Codex"}]
        assert verify_by_hint(str(tmp_project), hints) is True

    def test_pattern_not_found(self, tmp_project):
        target = tmp_project / "target.txt"
        target.write_text("hello world", encoding="utf-8")
        hints = [{"file": "target.txt", "pattern": "Codex"}]
        assert verify_by_hint(str(tmp_project), hints) is False

    def test_file_not_exists(self, tmp_project):
        hints = [{"file": "nonexistent.txt", "pattern": "anything"}]
        assert verify_by_hint(str(tmp_project), hints) is False

    def test_multiple_hints_all_match(self, tmp_project):
        (tmp_project / "a.txt").write_text("alpha content", encoding="utf-8")
        (tmp_project / "b.txt").write_text("beta content", encoding="utf-8")
        hints = [
            {"file": "a.txt", "pattern": "alpha"},
            {"file": "b.txt", "pattern": "beta"},
        ]
        assert verify_by_hint(str(tmp_project), hints) is True

    def test_multiple_hints_partial_match_fails(self, tmp_project):
        (tmp_project / "a.txt").write_text("alpha content", encoding="utf-8")
        (tmp_project / "b.txt").write_text("gamma content", encoding="utf-8")
        hints = [
            {"file": "a.txt", "pattern": "alpha"},
            {"file": "b.txt", "pattern": "beta"},
        ]
        assert verify_by_hint(str(tmp_project), hints) is False

    def test_empty_hints_returns_true(self, tmp_project):
        assert verify_by_hint(str(tmp_project), []) is True

    def test_hint_with_empty_file_or_pattern_skipped(self, tmp_project):
        hints = [{"file": "", "pattern": "x"}, {"file": "y", "pattern": ""}]
        assert verify_by_hint(str(tmp_project), hints) is True


class TestAuditWithHints:
    def test_hint_resolves_false_positive(self, tmp_project):
        """Core test: hint turns an unresolved action into resolved."""
        settled_dir = tmp_project / ".chanlun" / "genealogy" / "settled"
        _write_genealogy(settled_dir, "200", ["Update foo.txt with keyword XYZ"])

        # Without hints: should be unresolved
        report = audit(str(tmp_project))
        unresolved_items = [
            i for i in report["items"] if i["status"] == "unresolved"
        ]
        assert len(unresolved_items) == 1
        assert unresolved_items[0]["genealogy_id"] == "200"

        # Create the target file with the expected content
        (tmp_project / "foo.txt").write_text("contains keyword XYZ", encoding="utf-8")

        # Add verification hint
        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text(
            yaml.dump({"hints": {
                "200-1": [{"file": "foo.txt", "pattern": "XYZ"}],
            }}),
            encoding="utf-8",
        )

        # With hints: should now be resolved
        report2 = audit(str(tmp_project))
        assert report2["unresolved"] == 0
        assert report2["resolved"] == 1

    def test_hint_does_not_override_manual_override(self, tmp_project):
        """Manual overrides take precedence over hints."""
        settled_dir = tmp_project / ".chanlun" / "genealogy" / "settled"
        _write_genealogy(settled_dir, "201", ["Do something"])

        # Create override marking it as blocked (157号: long_term→blocked)
        overrides_path = tmp_project / ".chanlun" / "downstream-action-overrides.yaml"
        overrides_path.write_text("201-1: blocked\n", encoding="utf-8")

        # Create hint that would resolve it
        (tmp_project / "target.txt").write_text("match", encoding="utf-8")
        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text(
            yaml.dump({"hints": {
                "201-1": [{"file": "target.txt", "pattern": "match"}],
            }}),
            encoding="utf-8",
        )

        report = audit(str(tmp_project))
        # Override wins: status should be blocked, not resolved
        assert report["blocked"] == 1
        assert report["resolved"] == 0

    def test_no_hints_file_backward_compatible(self, tmp_project):
        """Without hints file, behavior is identical to original."""
        settled_dir = tmp_project / ".chanlun" / "genealogy" / "settled"
        _write_genealogy(settled_dir, "202", ["Something unresolved"])

        report = audit(str(tmp_project))
        assert report["unresolved"] == 1
        assert report["resolved"] == 0

    def test_hint_with_missing_target_stays_unresolved(self, tmp_project):
        """Hint exists but target file doesn't → stays unresolved."""
        settled_dir = tmp_project / ".chanlun" / "genealogy" / "settled"
        _write_genealogy(settled_dir, "203", ["Check nonexistent file"])

        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text(
            yaml.dump({"hints": {
                "203-1": [{"file": "does-not-exist.txt", "pattern": "anything"}],
            }}),
            encoding="utf-8",
        )

        report = audit(str(tmp_project))
        assert report["unresolved"] == 1


class TestVerifyByHintAbsent:
    """Tests for expect: absent support (156号扩展)."""

    def test_absent_pattern_not_in_file(self, tmp_project):
        """Pattern absent from file → hint satisfied."""
        (tmp_project / "target.txt").write_text("clean content", encoding="utf-8")
        hints = [{"file": "target.txt", "pattern": "forbidden", "expect": "absent"}]
        assert verify_by_hint(str(tmp_project), hints) is True

    def test_absent_pattern_in_file(self, tmp_project):
        """Pattern present in file but expect absent → hint fails."""
        (tmp_project / "target.txt").write_text("has forbidden word", encoding="utf-8")
        hints = [{"file": "target.txt", "pattern": "forbidden", "expect": "absent"}]
        assert verify_by_hint(str(tmp_project), hints) is False

    def test_absent_file_not_exists(self, tmp_project):
        """File doesn't exist → pattern can't be present → absent satisfied."""
        hints = [{"file": "nonexistent.txt", "pattern": "anything", "expect": "absent"}]
        assert verify_by_hint(str(tmp_project), hints) is True

    def test_mixed_present_and_absent(self, tmp_project):
        """Mix of present and absent hints all satisfied."""
        (tmp_project / "a.txt").write_text("has keyword", encoding="utf-8")
        (tmp_project / "b.txt").write_text("clean content", encoding="utf-8")
        hints = [
            {"file": "a.txt", "pattern": "keyword", "expect": "present"},
            {"file": "b.txt", "pattern": "forbidden", "expect": "absent"},
        ]
        assert verify_by_hint(str(tmp_project), hints) is True

    def test_mixed_present_and_absent_fails(self, tmp_project):
        """Present hint ok but absent hint fails → overall fails."""
        (tmp_project / "a.txt").write_text("has keyword", encoding="utf-8")
        (tmp_project / "b.txt").write_text("has forbidden too", encoding="utf-8")
        hints = [
            {"file": "a.txt", "pattern": "keyword", "expect": "present"},
            {"file": "b.txt", "pattern": "forbidden", "expect": "absent"},
        ]
        assert verify_by_hint(str(tmp_project), hints) is False

    def test_absent_default_is_present(self, tmp_project):
        """No expect field → defaults to present behavior."""
        (tmp_project / "target.txt").write_text("has content", encoding="utf-8")
        hints = [{"file": "target.txt", "pattern": "content"}]
        assert verify_by_hint(str(tmp_project), hints) is True


class TestAuditWithAbsentHints:
    """Integration test: absent hints resolve false positives (156号)."""

    def test_absent_hint_resolves_deletion_verification(self, tmp_project):
        """Core 156号 scenario: verify content was deleted from a file."""
        settled_dir = tmp_project / ".chanlun" / "genealogy" / "settled"
        _write_genealogy(settled_dir, "300", ["Remove forbidden pattern from config"])

        # Create the target file WITHOUT the forbidden pattern
        (tmp_project / "config.md").write_text("clean config", encoding="utf-8")

        # Add absent hint
        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text(
            yaml.dump({"hints": {
                "300-1": [{"file": "config.md", "pattern": "forbidden", "expect": "absent"}],
            }}),
            encoding="utf-8",
        )

        report = audit(str(tmp_project))
        assert report["unresolved"] == 0
        assert report["resolved"] == 1

    def test_absent_hint_fails_when_pattern_still_present(self, tmp_project):
        """Pattern still in file → absent hint does NOT resolve."""
        settled_dir = tmp_project / ".chanlun" / "genealogy" / "settled"
        _write_genealogy(settled_dir, "301", ["Remove forbidden from config"])

        # File still contains the forbidden pattern
        (tmp_project / "config.md").write_text("still has forbidden", encoding="utf-8")

        hints_path = tmp_project / ".chanlun" / "downstream-verification-hints.yaml"
        hints_path.write_text(
            yaml.dump({"hints": {
                "301-1": [{"file": "config.md", "pattern": "forbidden", "expect": "absent"}],
            }}),
            encoding="utf-8",
        )

        report = audit(str(tmp_project))
        assert report["unresolved"] == 1

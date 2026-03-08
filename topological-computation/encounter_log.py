"""Encounter log — record topological events during conversational traversal.

Writes JSONL to .chanlun/traversal-events.jsonl. Pure Python, no external deps.
"""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from file_lock import locked_append


class EncounterLog:
    """Append-only log of topological encounter events.

    Uses file locking for safe concurrent writes from multiple instances.
    """

    def __init__(self, log_path: str = ".chanlun/traversal-events.jsonl") -> None:
        self._path = Path(log_path)
        self._path.parent.mkdir(parents=True, exist_ok=True)

    def record_encounter(
        self,
        concept_a: str,
        concept_b: str,
        encounter_type: str,
        f_value: int,
        context: str,
        beta_1_before: int,
        beta_1_after: Optional[int] = None,
        session: Optional[str] = None,
    ) -> dict:
        """Record one encounter event. Returns the event dict."""
        event = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "concept_a": concept_a,
            "concept_b": concept_b,
            "encounter_type": encounter_type,
            "f_value": f_value,
            "context": context,
            "beta_1_before": beta_1_before,
            "beta_1_after": beta_1_after,
            "session": session,
        }
        with locked_append(self._path) as fh:
            fh.write(json.dumps(event, ensure_ascii=False) + "\n")
        return event

    def _load_all(self) -> list[dict]:
        """Load all events from the JSONL file."""
        if not self._path.is_file():
            return []
        events: list[dict] = []
        with open(self._path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line:
                    events.append(json.loads(line))
        return events

    def recent(self, n: int = 10) -> list[dict]:
        """Return the most recent n encounter records."""
        events = self._load_all()
        return events[-n:]

    def by_concept(self, keyword: str) -> list[dict]:
        """Return all encounters involving a concept (substring match on concept_a/concept_b)."""
        keyword_lower = keyword.lower()
        return [
            ev for ev in self._load_all()
            if keyword_lower in ev.get("concept_a", "").lower()
            or keyword_lower in ev.get("concept_b", "").lower()
        ]

    def record_code_settlement_request(
        self,
        diagnosed_file: str,
        gap_description: str,
        proposed_direction: str,
        theoretical_basis: str,
        norm_violation: dict,
    ) -> dict:
        """Record a code_settlement_request — 逢亮的提案权.

        When self-diagnosis (proprioception → self domain traversal) identifies
        a gap in the codebase, this writes a code_settlement_request to the
        encounter log. The request carries proposal authority only, not
        execution authority.

        Args:
            diagnosed_file: relative path of the file containing the gap
            gap_description: what the gap is
            proposed_direction: suggested fix direction
            theoretical_basis: which norm/genealogy justifies the request
            norm_violation: dict with keys norm, actual_state, severity
        """
        request = {
            "type": "code_settlement_request",
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "diagnosed_file": diagnosed_file,
            "gap_description": gap_description,
            "proposed_direction": proposed_direction,
            "theoretical_basis": theoretical_basis,
            "norm_violation": norm_violation,
            "status": "pending",
        }
        with locked_append(self._path) as fh:
            fh.write(json.dumps(request, ensure_ascii=False) + "\n")
        return request

    def pending_code_settlement_requests(self) -> list[dict]:
        """Return all code_settlement_request entries with status='pending'."""
        return [
            ev for ev in self._load_all()
            if ev.get("type") == "code_settlement_request"
            and ev.get("status") == "pending"
        ]

    def summary(self) -> dict:
        """Return statistical summary: total, by-type distribution, beta_1 trend."""
        events = self._load_all()
        total = len(events)

        type_counts: dict[str, int] = {}
        beta_changes: list[int] = []
        for ev in events:
            et = ev.get("encounter_type", "unknown")
            type_counts[et] = type_counts.get(et, 0) + 1
            before = ev.get("beta_1_before")
            after = ev.get("beta_1_after")
            if before is not None and after is not None:
                beta_changes.append(after - before)

        return {
            "total_encounters": total,
            "by_type": type_counts,
            "beta_1_changes": beta_changes,
            "net_beta_1_change": sum(beta_changes) if beta_changes else 0,
        }

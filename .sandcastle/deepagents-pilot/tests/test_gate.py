"""门的正负控。这里的批准记录是测试内合成的，只证明纯判定逻辑，不计入任何真实人批。"""
from pilot1742.domain import (
    ApprovalRecord,
    Approved,
    Awaiting,
    HumanSource,
    Refused,
    ReviewVerdict,
    decide_gate,
    parse_sha,
)
from pilot1742.run import parse_approval_ledger

H1 = parse_sha("1" * 40)
H2 = parse_sha("2" * 40)
SYNTHETIC = HumanSource("user_message", "SYNTHETIC-TEST-ONLY", "pytest")
PASS_H2 = ReviewVerdict(H2, "PASS", "gpt-6.1-sol", "claude-opus-5-5")


def record(seq, head, decision="approve", scope="pilot_acceptance", ticket=1742):
    return ApprovalRecord(seq, ticket, scope, head, decision, SYNTHETIC)


def gate(records, verdict=PASS_H2, scope="pilot_acceptance", candidate=H2):
    return decide_gate(ticket=1742, scope=scope, candidate=candidate, verdict=verdict, records=records)


def test_same_head_approval_passes():
    assert isinstance(gate([record(1, H2)]), Approved)


def test_approval_for_old_head_does_not_carry_to_new_head():
    decision = gate([record(1, H1)])
    assert isinstance(decision, Awaiting) and decision.reasons == ("approval_for_other_head",)


def test_later_reject_revokes_and_headless_reject_applies_to_all():
    assert isinstance(gate([record(1, H2), record(2, H2, "reject")]), Refused)
    assert gate([record(1, H2), record(2, None, "reject")]).reasons == ("revoked",)
    assert isinstance(gate([record(1, H2, "reject"), record(2, H2)]), Approved)


def test_scope_and_ticket_do_not_cross():
    assert isinstance(gate([record(1, H2, scope="main_merge")]), Awaiting)
    assert isinstance(gate([record(1, H2)], scope="main_merge"), Awaiting)
    assert isinstance(gate([record(1, H2, ticket=1709)]), Awaiting)


def test_review_must_be_cross_family_pass_at_the_candidate_head():
    assert gate([record(1, H2)], verdict=None).reasons == ("no_review_at_head",)
    assert gate([record(1, H2)], verdict=ReviewVerdict(H1, "PASS", "gpt-6.1-sol", "claude-opus-5-5")).reasons \
        == ("no_review_at_head",)
    assert gate([record(1, H2)], verdict=ReviewVerdict(H2, "BLOCK", "gpt-6.1-sol", "claude-opus-5-5")).reasons \
        == ("review_block",)
    assert gate([record(1, H2)], verdict=ReviewVerdict(H2, "VERIFIER_BLOCKED", "gpt-6.1-sol", "gpt-6.1-sol")).reasons \
        == ("verifier_blocked",)


def test_ledger_counts_only_registered_human_sources():
    url = "https://github.com/xy7365527-lang/NewChanlun/issues/1742#issuecomment-123"
    good = f'{{"ticket":1742,"scope":"pilot_acceptance","head":"{H2}","decision":"approve",' \
           f'"source":{{"kind":"tracker_comment","reference":"{url}","registered_by":"主控"}}}}'
    bad = [
        f'{{"ticket":1742,"scope":"pilot_acceptance","head":"{H2}","decision":"approve",'
        f'"source":{{"kind":"synthetic_test","reference":"x","registered_by":"pytest"}}}}',
        f'{{"ticket":1742,"scope":"pilot_acceptance","head":"{H2}","decision":"approve",'
        f'"source":{{"kind":"tracker_comment","reference":"https://github.com/xy7365527-lang/NewChanlun/issues/1709#issuecomment-1","registered_by":"主控"}}}}',
        f'{{"ticket":1742,"scope":"pilot_acceptance","head":"{H2}","decision":"approve",'
        f'"source":{{"kind":"user_message","reference":" ","registered_by":"主控"}}}}',
        '{"ticket":1742,"scope":"pilot_acceptance","head":null,"decision":"approve",'
        '"source":{"kind":"user_message","reference":"msg","registered_by":"主控"}}',
        f'{{"ticket":1742,"scope":"pilot_acceptance","head":"{H2[:12]}","decision":"approve",'
        f'"source":{{"kind":"user_message","reference":"msg","registered_by":"主控"}}}}',
    ]
    records, problems = parse_approval_ledger("\n".join([good, *bad]))
    assert [r.source.reference for r in records] == [url]
    assert len(problems) == len(bad)
    records, _ = parse_approval_ledger("\n".join(bad))
    assert isinstance(gate(records), Awaiting)

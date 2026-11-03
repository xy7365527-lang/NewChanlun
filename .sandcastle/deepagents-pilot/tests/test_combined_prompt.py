from dataclasses import replace

import pytest
from conftest import DESIGN_TASK

from pilot1742.domain import Terminal, WorkerKey
from pilot1742.run import SKILLS_ROOT

SERVICE = "你由另一个代理作为工具调用，只向主控交回结果，不直接与用户对话。"
CLOUD_SKILLS = "/opt/newchanlun/pstack/installed/skills/"


@pytest.mark.parametrize("cloud", [False, True])
def test_design_and_judge_keep_backend_paths_and_each_candidates_limits(env, cloud):
    run = env.create(task=DESIGN_TASK)
    env.control({
        "design-a": {"summary": "设计甲", "findings": ["甲拒绝项"], "validation": ["甲核查"]},
        "design-b": {"summary": "设计乙", "findings": ["乙拒绝项"], "validation": ["乙核查"]},
    })
    receipts = run.run_design("同题设计")
    facts = run.facts()
    for receipt, reason in zip(receipts, ("甲未查远端", "乙未做压测"), strict=True):
        facts.workers[receipt.key] = Terminal(replace(receipt, failure_reason=reason))
    run.execution = object() if cloud else None
    root = CLOUD_SKILLS if cloud else SKILLS_ROOT
    for receipt in receipts:
        prompt = run._prompt(receipt.key, "同题设计", facts)
        assert prompt.startswith(SERVICE)
        for name in ("SKILL.md", "references/runner-prompt.md", "references/rationale-template.md"):
            assert root + "architect/" + name in prompt
        if cloud:
            assert SKILLS_ROOT not in prompt
    judge = run._prompt(WorkerKey(run.spec.run_id, "judge", env.base), "按原 rubric", facts)
    assert judge.startswith(SERVICE)
    assert root + "arena/SKILL.md" in judge
    for label, summary, rejected, validation, reason in (
        ("候选 1", "设计甲", "甲拒绝项", "甲核查", "甲未查远端"),
        ("候选 2", "设计乙", "乙拒绝项", "乙核查", "乙未做压测"),
    ):
        package = judge.split("## " + label + "\n", 1)[1].split("\n## 候选", 1)[0]
        assert all(value in package for value in (summary, rejected, validation, reason))
        assert "未单独采集 confidence 与 caveats" in package
    if cloud:
        assert SKILLS_ROOT not in judge
    run.execution = None
    run.release()


@pytest.mark.parametrize("cloud", [False, True])
def test_author_rework_keeps_review_validation_and_failure_reason(env, cloud):
    run = env.create()
    env.control({"author": {"write": {"demo/a.py": "x = 1\n"}},
                 "review": {"summary": "VERDICT: BLOCK\n缺边界", "findings": ["补空值测试"],
                            "validation": ["只跑本地靶向"]}})
    run.run_worker("author", "实现")
    head = run.commit_candidate().commit
    review = run.run_worker("review", "独审")
    facts = run.facts()
    facts.workers[review.key] = Terminal(replace(review, failure_reason="未验真实 provider"))
    run.execution = object() if cloud else None
    prompt = run._prompt(WorkerKey(run.spec.run_id, "author", head), "按独审返修", facts)
    assert prompt.startswith(SERVICE)
    for value in ("VERDICT: BLOCK", "补空值测试", "只跑本地靶向", "未验真实 provider",
                  "未单独采集 confidence 与 caveats"):
        assert value in prompt
    review_prompt = run._prompt(WorkerKey(run.spec.run_id, "review", head), "独审", facts)
    assert review_prompt.startswith(SERVICE)
    assert head in review_prompt
    run.execution = None
    run.release()

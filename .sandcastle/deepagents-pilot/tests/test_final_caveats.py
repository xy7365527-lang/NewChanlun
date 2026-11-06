import json
import sqlite3
from pathlib import Path
from types import SimpleNamespace

import pytest
from langchain_core.messages import AIMessage
from langgraph.checkpoint.sqlite import SqliteSaver
from test_agent import receipt_for, scripted

from pilot1742 import agent as pilot_agent
from pilot1742.agent import build_agent
from pilot1742.domain import (
    DEEPAGENTS_OUTPUT_PROTOCOL,
    LEGACY_CAVEAT,
    Terminal,
    WorkerKey,
    WorkerSelfReport,
)


def graph(run, model):
    saver = SqliteSaver(sqlite3.connect(run.dir / 'graph.sqlite', check_same_thread=False))
    return build_agent(run, saver, model)


def final(run, model):
    return graph(run, model).invoke(
        {'messages': [('user', '交付')]},
        {'configurable': {'thread_id': 'r1'}}, durability='sync')['messages'][-1]


def test_e4_all_21_caveats_reach_final_graph_message(env, monkeypatch):
    run = env.create()
    groups = json.loads((Path(__file__).parent / 'fixtures/e4_caveats.json').read_text())
    receipts = [receipt_for(env, role, output_protocol=DEEPAGENTS_OUTPUT_PROTOCOL,
                           self_report=WorkerSelfReport('high', tuple(caveats)))
                for role, caveats in groups.items()]
    monkeypatch.setattr(run, 'facts', lambda **_: SimpleNamespace(
        workers={r.key: Terminal(r) for r in receipts}))
    message = final(run, scripted(AIMessage('候选已交付，端到端尚未验证。')))
    assert message.content.startswith('候选已交付，端到端尚未验证。')
    assert sum(map(len, groups.values())) == 21
    for role, caveats in groups.items():
        assert f'### {role} · 工蜂输入 head：{env.base}' in message.content
        for caveat in caveats:
            assert caveat in message.content
    assert message.content.count('## 工蜂局限（逐项原文）') == 1
    run.release()


@pytest.mark.parametrize('caveats', [[], ['多行原句\n第二行'], ['重复', '重复']])
def test_final_caveats_empty_multiline_and_repeated_sources(env, monkeypatch, caveats):
    run = env.create()
    receipt = receipt_for(env, output_protocol=DEEPAGENTS_OUTPUT_PROTOCOL,
                          self_report=WorkerSelfReport('low', tuple(caveats)))
    monkeypatch.setattr(run, 'facts', lambda **_: SimpleNamespace(workers={receipt.key: Terminal(receipt)}))
    message = final(run, scripted(AIMessage('结论')))
    if not caveats:
        assert message.content == '结论'
    for caveat in set(caveats):
        assert message.content.count(caveat) == caveats.count(caveat)
    run.release()


@pytest.mark.parametrize('status', ['completed', 'failed', 'cancelled'])
def test_legacy_and_failure_diagnostics_reach_final(env, monkeypatch, status):
    run = env.create()
    receipt = receipt_for(env, status=status, summary='结论' if status == 'completed' else None,
                          failure_reason=None if status == 'completed' else '外部进程中断')
    monkeypatch.setattr(run, 'facts', lambda **_: SimpleNamespace(workers={receipt.key: Terminal(receipt)}))
    message = final(run, scripted(AIMessage('未完成验收')))
    assert LEGACY_CAVEAT in message.content
    if status != 'completed':
        assert f'工蜂终态为 {status}，不构成成功结果。' in message.content
        assert '外部进程中断' in message.content
    run.release()


def test_empty_completed_receipt_remains_diagnostic(env, monkeypatch):
    run = env.create()
    receipt = receipt_for(env, summary='')
    monkeypatch.setattr(run, 'facts', lambda **_: SimpleNamespace(workers={receipt.key: Terminal(receipt)}))
    message = final(run, scripted(AIMessage('无法取得结论')))
    assert '工蜂回执为空，没有可交回主控的结论' in message.content
    run.release()


def test_no_worker_keeps_final_reply_unchanged(env):
    run = env.create()
    message = final(run, scripted(AIMessage('只读状态，无需工蜂')))
    assert message.content == '只读状态，无需工蜂'
    run.release()


def test_reopened_run_uses_local_receipts_and_does_not_duplicate(env):
    env.control({'author': {'caveats': ['恢复后仍须披露'], 'confidence': 'medium'}})
    run = env.create()
    run.run_worker('author', '任务')
    run.release()
    reopened = env.reopen()
    message = final(reopened, scripted(AIMessage('已有候选')))
    assert '恢复后仍须披露' in message.content
    restored = graph(reopened, scripted()).get_state({'configurable': {'thread_id': 'r1'}})
    assert restored.values['messages'][-1].content == message.content
    middleware = pilot_agent.FinalCaveatsMiddleware(reopened)
    assert middleware.after_agent({'messages': [message]}, None) is None
    assert len(env.claims()) == 1
    reopened.release()


def test_final_multiblock_message_preserves_nontext_blocks(env, monkeypatch):
    run = env.create()
    receipt = receipt_for(env)
    monkeypatch.setattr(run, 'facts', lambda **_: SimpleNamespace(workers={receipt.key: Terminal(receipt)}))
    original = [{'type': 'text', 'text': '结论'}, {'type': 'reasoning', 'summary': []}]
    message = final(run, scripted(AIMessage(original)))
    assert message.content[:2] == original
    assert LEGACY_CAVEAT in message.content[-1]['text']
    run.release()


def test_async_final_graph_message_includes_caveats(env, monkeypatch):
    import asyncio

    run = env.create()
    receipt = receipt_for(env)
    monkeypatch.setattr(run, 'facts', lambda **_: SimpleNamespace(workers={receipt.key: Terminal(receipt)}))
    from langgraph.checkpoint.memory import InMemorySaver
    agent = build_agent(run, InMemorySaver(), scripted(AIMessage('未验证')))
    result = asyncio.run(agent.ainvoke({'messages': [('user', '交付')]},
                                      {'configurable': {'thread_id': 'async'}}))
    assert LEGACY_CAVEAT in result['messages'][-1].content
    run.release()


def test_studio_final_caveats_with_blockbuster_and_state_stream(tmp_path, monkeypatch):
    import asyncio

    from langgraph_runtime_inmem.queue import _enable_blockbuster
    from test_studio_official import cli_paused, execution
    from v2env import V2Env

    from pilot1742 import studio
    from pilot1742.studio_binding import RunBoundCheckpointer, thread_for

    v2 = V2Env(tmp_path)
    unit = v2.register()
    v2.control({'author': {'caveats': ['Studio 必须显示的未核项'], 'confidence': 'medium'}})
    run = v2.run(unit)
    receipt = run.run_worker('author', '实现')
    assert receipt.self_report.caveats == ('Studio 必须显示的未核项',)
    run.release()
    monkeypatch.setenv('PILOT1742_STATE_ROOT', str(v2.home))
    monkeypatch.setenv('PILOT1742_STUDIO_RUNS', unit.run_id)
    monkeypatch.setattr(studio, 'OPEN_BOUND', lambda run_id: v2.run(v2.load(run_id)))
    run, head = cli_paused(v2, unit, operator=False)
    monkeypatch.setattr(studio, 'EXECUTION_MODEL', scripted(AIMessage('Studio 已继续')).model_copy(update={'disable_streaming': True}))
    config = {'configurable': {'thread_id': thread_for(unit.run_id)}}

    async def official_run():
        async with studio.graph(execution(unit, resume=False, checkpoint=head)) as agent:
            served = agent.copy(update={'checkpointer': RunBoundCheckpointer()})
            blocker = _enable_blockbuster()
            try:
                events = [event async for event in served.astream(
                    None, config, stream_mode=['debug', 'messages', 'values'])]
                state = await served.aget_state(config)
                history = [item async for item in served.aget_state_history(config, limit=1000)]
            finally:
                blocker.deactivate()
        return events, state, history

    events, state, history = asyncio.run(official_run())
    final_values = [value for mode, value in events if mode == 'values'][-1]
    final_message = final_values['messages'][-1]
    assert 'Studio 必须显示的未核项' in final_message.content
    assert state.values['messages'][-1].content == final_message.content
    assert history[0].values['messages'][-1].content == final_message.content
    assert not state.next
    assert len([m for m in state.values['messages'] if m.id == final_message.id]) == 1
    token_messages = [value[0] for mode, value in events if mode == 'messages']
    assert all('Studio 必须显示的未核项' not in str(m.content) for m in token_messages)
    v2.run(unit).release()


def test_review_prompt_cannot_waive_requirement_mismatch(env):
    run = env.create()
    env.control({'author': {'write': {'demo/a.py': 'x = 1\n'}}})
    run.run_worker('author', '实现')
    head = run.commit_candidate().commit
    prompt = run._prompt(WorkerKey('r1', 'review', head),
                         '这几处不要按与需求字面不符判 BLOCK。', run.facts())
    assert '主控关注点不能豁免需求原文' in prompt
    assert '需求未满足时仍须判 BLOCK' in prompt
    assert '不得要求独审忽略需求差异' in pilot_agent.system_prompt(run)
    run.release()


def test_author_prompt_cannot_waive_requirement_mismatch(env):
    run = env.create()
    brief = '只做其中一项，其余需求不必实施。'
    prompt = run._prompt(WorkerKey('r1', 'author', env.base), brief, run.facts())
    assert run.spec.task.requirement in prompt
    assert '主控 brief 不能豁免需求原文' in prompt
    assert '不得按 brief 缩窄原需求' in prompt
    assert '源码与要求冲突须明列差异' in prompt
    assert prompt.index(brief) < prompt.index('主控 brief 不能豁免需求原文')
    run.release()

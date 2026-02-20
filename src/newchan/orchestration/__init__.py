"""事件路由表——读取 dispatch-spec.yaml 的 orchestration_protocol 段并匹配事件。"""

from newchan.orchestration.router import Route, load_routes, match_event

__all__ = ["Route", "load_routes", "match_event"]

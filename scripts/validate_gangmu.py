#!/usr/bin/env python3
"""验证 .chanlun/gangmu.yaml 的结构合法性。

退出码：0=合法，1=不合法（打印所有错误）。
"""

import sys
from pathlib import Path

import yaml

GANGMU_PATH = Path(__file__).resolve().parent.parent / ".chanlun" / "gangmu.yaml"

VALID_STATUS = {"active", "blocked", "closed"}
VALID_ACTION_TYPE = {"engineering", "experiment", "theory", "integration"}
VALID_CHECK_TYPE = {"file_exists", "genealogy_settled", "script_exists", "test_pass"}
CHECK_TYPE_REQUIRED_FIELDS = {
    "file_exists": ["path"],
    "genealogy_settled": ["keyword"],
    "script_exists": ["path"],
    "test_pass": ["pattern"],
}


def validate(data: object) -> list[str]:
    errors: list[str] = []

    if not isinstance(data, dict):
        return ["顶层不是映射"]

    # 规则1：顶层必须有 version, zongfangzhen_ref, gang
    for key in ("version", "zongfangzhen_ref", "gang"):
        if key not in data:
            errors.append(f"顶层缺少必需字段: {key}")

    gang_list = data.get("gang")
    if not isinstance(gang_list, list):
        errors.append("gang 不是列表")
        return errors

    gang_ids: set[str] = set()
    mu_ids: set[str] = set()

    for gi, gang in enumerate(gang_list):
        prefix = f"gang[{gi}]"
        if not isinstance(gang, dict):
            errors.append(f"{prefix} 不是映射")
            continue

        # 规则2：gang 必需字段
        for key in ("id", "name", "zongfangzhen_section", "description", "mu"):
            if key not in gang:
                errors.append(f"{prefix} 缺少必需字段: {key}")

        gang_id = gang.get("id")

        # 规则10：gang id 唯一
        if gang_id is not None:
            if gang_id in gang_ids:
                errors.append(f"{prefix} gang id 重复: {gang_id}")
            gang_ids.add(gang_id)

        mu_list = gang.get("mu")
        if mu_list is None:
            continue
        if not isinstance(mu_list, list):
            errors.append(f"{prefix}.mu 不是列表")
            continue

        for mi, mu in enumerate(mu_list):
            mu_prefix = f"{prefix}.mu[{mi}]"
            if not isinstance(mu, dict):
                errors.append(f"{mu_prefix} 不是映射")
                continue

            # 规则3：mu 必需字段
            for key in ("id", "name", "status", "opened_at", "next_actions"):
                if key not in mu:
                    errors.append(f"{mu_prefix} 缺少必需字段: {key}")

            mu_id = mu.get("id")

            # 规则9：mu id 全局唯一
            if mu_id is not None:
                if mu_id in mu_ids:
                    errors.append(f"{mu_prefix} mu id 全局重复: {mu_id}")
                mu_ids.add(mu_id)

            status = mu.get("status")

            # 规则4：status 枚举
            if status is not None and status not in VALID_STATUS:
                errors.append(
                    f"{mu_prefix} status 非法: {status!r}（允许: {VALID_STATUS}）"
                )

            next_actions = mu.get("next_actions")

            # 规则11：closed 的 mu 的 next_actions 必须是空列表
            if status == "closed":
                if not isinstance(next_actions, list) or len(next_actions) != 0:
                    errors.append(
                        f"{mu_prefix} closed 状态的 mu 的 next_actions 必须是空列表"
                    )

            # 规则12：blocked 的 mu 必须有 blocked_by
            if status == "blocked" and "blocked_by" not in mu:
                errors.append(
                    f"{mu_prefix} blocked 状态的 mu 必须有 blocked_by 字段"
                )

            if next_actions is None:
                continue
            if not isinstance(next_actions, list):
                errors.append(f"{mu_prefix}.next_actions 不是列表")
                continue

            for ai, action in enumerate(next_actions):
                a_prefix = f"{mu_prefix}.next_actions[{ai}]"
                if not isinstance(action, dict):
                    errors.append(f"{a_prefix} 不是映射")
                    continue

                # 规则5：next_actions 必需字段
                for key in (
                    "type",
                    "target",
                    "description",
                    "blocked_by",
                    "completion_check",
                ):
                    if key not in action:
                        errors.append(f"{a_prefix} 缺少必需字段: {key}")

                action_type = action.get("type")

                # 规则6：type 枚举
                if action_type is not None and action_type not in VALID_ACTION_TYPE:
                    errors.append(
                        f"{a_prefix} type 非法: {action_type!r}"
                        f"（允许: {VALID_ACTION_TYPE}）"
                    )

                cc = action.get("completion_check")
                if cc is None:
                    continue
                if not isinstance(cc, dict):
                    errors.append(f"{a_prefix}.completion_check 不是映射")
                    continue

                # 规则7：completion_check 必须有 type
                cc_type = cc.get("type")
                if cc_type is None:
                    errors.append(f"{a_prefix}.completion_check 缺少 type 字段")
                elif cc_type not in VALID_CHECK_TYPE:
                    errors.append(
                        f"{a_prefix}.completion_check type 非法: {cc_type!r}"
                        f"（允许: {VALID_CHECK_TYPE}）"
                    )
                else:
                    # 规则8：对应 type 的必需字段
                    for req in CHECK_TYPE_REQUIRED_FIELDS[cc_type]:
                        if req not in cc:
                            errors.append(
                                f"{a_prefix}.completion_check (type={cc_type})"
                                f" 缺少必需字段: {req}"
                            )

    return errors


def main() -> int:
    if not GANGMU_PATH.exists():
        print(f"ERROR: 文件不存在: {GANGMU_PATH}", file=sys.stderr)
        return 1

    with open(GANGMU_PATH, encoding="utf-8") as f:
        data = yaml.safe_load(f)

    errors = validate(data)

    if errors:
        print(f"gangmu.yaml 验证失败 — {len(errors)} 个错误:\n")
        for i, e in enumerate(errors, 1):
            print(f"  [{i}] {e}")
        return 1

    print("gangmu.yaml 验证通过 — 0 个错误")
    return 0


if __name__ == "__main__":
    sys.exit(main())

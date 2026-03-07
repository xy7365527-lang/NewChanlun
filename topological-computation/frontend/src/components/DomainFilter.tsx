/**
 * DomainFilter.tsx
 *
 * 域过滤栏组件。
 * 渲染一排按钮（全部 + 各域），支持多选。
 * 过滤是纯前端行为——不重新请求 API。
 */

import { T, FONT } from "../tokens";
import { DOMAIN_FILTERS, type DomainKey } from "../utils/classifyNode";

interface Props {
  activeFilters: Set<DomainKey> | null; // null = 全部（不过滤）
  onChange: (filters: Set<DomainKey> | null) => void;
}

export function DomainFilter({ activeFilters, onChange }: Props) {
  const isAll = activeFilters === null;

  function handleAll() {
    onChange(null);
  }

  function handleDomain(key: DomainKey) {
    if (isAll) {
      // 从"全部"切换到单域选中
      onChange(new Set([key]));
      return;
    }
    const next = new Set(activeFilters!);
    if (next.has(key)) {
      next.delete(key);
      // 如果全部取消选中，回到"全部"
      if (next.size === 0) {
        onChange(null);
      } else {
        onChange(next);
      }
    } else {
      next.add(key);
      onChange(next);
    }
  }

  return (
    <div style={{
      display: "flex",
      alignItems: "center",
      gap: 4,
      padding: "4px 8px",
      borderBottom: `1px solid ${T.border}`,
      background: T.bgPanel,
      flexShrink: 0,
      overflowX: "auto",
      flexWrap: "nowrap",
    }}>
      {/* "全部" 按钮 */}
      <FilterButton
        label="全部"
        color="#9ca3af"
        active={isAll}
        onClick={handleAll}
      />

      <div style={{
        width: 1, height: 14,
        background: T.border,
        marginInline: 2,
        flexShrink: 0,
      }} />

      {/* 各域按钮 */}
      {DOMAIN_FILTERS.map((meta) => {
        const active = !isAll && activeFilters!.has(meta.key);
        return (
          <FilterButton
            key={meta.key}
            label={meta.label}
            color={meta.color}
            active={active}
            dimmed={!isAll && !active}
            onClick={() => handleDomain(meta.key)}
          />
        );
      })}
    </div>
  );
}

// ── 单个过滤按钮 ──────────────────────────────────────────────────

interface ButtonProps {
  label: string;
  color: string;
  active: boolean;
  dimmed?: boolean;
  onClick: () => void;
}

function FilterButton({ label, color, active, dimmed = false, onClick }: ButtonProps) {
  return (
    <button
      onClick={onClick}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 5,
        padding: "2px 8px",
        borderRadius: 4,
        border: active
          ? `1px solid ${color}88`
          : `1px solid ${T.border}`,
        background: active
          ? `${color}18`
          : "transparent",
        cursor: "pointer",
        fontFamily: FONT.mono,
        fontSize: 10,
        letterSpacing: "0.04em",
        color: active
          ? color
          : dimmed
            ? T.textMuted
            : T.textDim,
        opacity: dimmed ? 0.45 : 1,
        transition: "all 0.12s",
        whiteSpace: "nowrap",
        flexShrink: 0,
      }}
    >
      {/* 色点 */}
      <span style={{
        width: 6, height: 6,
        borderRadius: "50%",
        background: color,
        opacity: active ? 1 : 0.4,
        flexShrink: 0,
      }} />
      {label}
    </button>
  );
}

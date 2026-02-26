/**
 * LevelSelector — 递归级别切换 UI
 *
 * 纯前端过滤：从 overlay 数据中读取可用级别数，
 * 用户选择后通过 appStore.selectedLevel 控制显示哪些级别的中枢。
 * level=0 表示显示所有级别。
 */
import { type CSSProperties } from "react";
import { useAppStore } from "../store/appStore";

const wrapStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 2,
};

const labelStyle: CSSProperties = {
  fontSize: 11,
  color: "#787b86",
  marginRight: 2,
};

const btnStyle = (active: boolean): CSSProperties => ({
  fontSize: 11,
  height: 22,
  minWidth: 28,
  background: active ? "#2962ff" : "transparent",
  color: active ? "#fff" : "#787b86",
  border: "none",
  padding: "0 5px",
  borderRadius: 3,
  cursor: "pointer",
});

interface LevelSelectorProps {
  maxLevel: number;
}

export function LevelSelector({ maxLevel }: LevelSelectorProps) {
  const { selectedLevel, setSelectedLevel } = useAppStore();

  if (maxLevel <= 0) return null;

  const levels = [0]; // 0 = All
  for (let i = 1; i <= maxLevel; i++) {
    levels.push(i);
  }

  return (
    <div style={wrapStyle}>
      <span style={labelStyle}>Lv</span>
      {levels.map((lv) => (
        <button
          key={lv}
          style={btnStyle(selectedLevel === lv)}
          onClick={() => setSelectedLevel(lv)}
          title={lv === 0 ? "显示所有级别" : `仅显示 Level ${lv}`}
        >
          {lv === 0 ? "All" : `${lv}`}
        </button>
      ))}
    </div>
  );
}

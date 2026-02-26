/**
 * SymbolSelector — 标的选择下拉菜单
 *
 * 从 GET /api/symbols 获取已缓存品种列表，
 * 切换时更新 appStore.symbol，触发 WebSocket 重连。
 */
import { useState, useEffect, useRef, type CSSProperties } from "react";
import { getSymbols } from "../api/client";
import { useAppStore } from "../store/appStore";

const wrapStyle: CSSProperties = {
  position: "relative",
  display: "inline-block",
};

const btnStyle: CSSProperties = {
  height: 26,
  fontSize: 12,
  background: "#2a2e39",
  border: "1px solid #363a45",
  color: "#d1d4dc",
  padding: "0 10px",
  borderRadius: 3,
  cursor: "pointer",
  display: "flex",
  alignItems: "center",
  gap: 4,
};

const dropdownStyle: CSSProperties = {
  position: "absolute",
  top: 28,
  left: 0,
  width: 180,
  maxHeight: 260,
  overflowY: "auto",
  background: "#1e222d",
  border: "1px solid #363a45",
  borderRadius: 4,
  zIndex: 300,
  boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
};

const itemStyle: CSSProperties = {
  padding: "6px 10px",
  cursor: "pointer",
  fontSize: 12,
  borderBottom: "1px solid #2a2e39",
  color: "#d1d4dc",
};

const itemHoverBg = "#2a2e39";

interface SymbolItem {
  symbol: string;
  interval: string;
}

export function SymbolSelector() {
  const { symbol, setSymbol } = useAppStore();
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<SymbolItem[]>([]);
  const [hoveredIdx, setHoveredIdx] = useState(-1);
  const wrapRef = useRef<HTMLDivElement>(null);

  // 加载品种列表
  useEffect(() => {
    let cancelled = false;
    getSymbols()
      .then((data) => {
        if (!cancelled) setItems(data);
      })
      .catch(() => {});
    return () => { cancelled = true; };
  }, []);

  // 点击外部关闭
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("click", handleClick);
    return () => document.removeEventListener("click", handleClick);
  }, []);

  function handleSelect(sym: string) {
    setSymbol(sym);
    setOpen(false);
  }

  return (
    <div ref={wrapRef} style={wrapStyle}>
      <button
        style={btnStyle}
        onClick={() => setOpen(!open)}
        title="切换标的"
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        {symbol} <span style={{ fontSize: 10, color: "#787b86" }}>&#9662;</span>
      </button>
      {open && (
        <div style={dropdownStyle} role="listbox" aria-label="标的列表">
          {items.length === 0 && (
            <div style={{ ...itemStyle, color: "#787b86" }}>无缓存品种</div>
          )}
          {items.map((it, i) => (
            <div
              key={`${it.symbol}-${it.interval}`}
              role="option"
              aria-selected={it.symbol === symbol}
              style={{
                ...itemStyle,
                background: hoveredIdx === i ? itemHoverBg : "transparent",
                fontWeight: it.symbol === symbol ? 700 : 400,
                color: it.symbol === symbol ? "#2962ff" : "#d1d4dc",
              }}
              onClick={() => handleSelect(it.symbol)}
              onMouseEnter={() => setHoveredIdx(i)}
              onMouseLeave={() => setHoveredIdx(-1)}
            >
              {it.symbol}
              <span style={{ marginLeft: 8, color: "#787b86", fontSize: 10 }}>
                {it.interval}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

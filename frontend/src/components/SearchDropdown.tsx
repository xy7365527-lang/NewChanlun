import { useState, useRef, useEffect } from "react";
import { useSearch } from "../hooks/useSearch";
import { useAppStore } from "../store/appStore";

export function SearchDropdown() {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const { results } = useSearch(query);
  const setSymbol = useAppStore((s) => s.setSymbol);
  const wrapRef = useRef<HTMLDivElement>(null);

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

  function selectSymbol(sym: string) {
    setSymbol(sym);
    setQuery("");
    setOpen(false);
  }

  return (
    <div ref={wrapRef} className="search-wrap">
      <input
        className="search-input"
        placeholder="搜索品种 (BZ, CL, gold, 黄金...)"
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
        }}
        onFocus={() => query && setOpen(true)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && query.trim()) {
            selectSymbol(query.trim());
          }
          if (e.key === "Escape") setOpen(false);
        }}
      />
      {open && results.length > 0 && (
        <div className="search-dropdown">
          {results.map((r, i) => (
            <div
              key={`${r.symbol}-${r.source}-${i}`}
              className="search-item"
              onClick={() => selectSymbol(r.symbol)}
            >
              <span className="search-sym">{r.symbol}</span>
              <span className="search-desc">{r.description}</span>
              <span className={`search-badge ${r.source}`}>
                {r.source === "cache" ? "缓存" : r.exchange}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

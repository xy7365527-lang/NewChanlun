import { useState, useEffect, useRef } from "react";
import { searchSymbols } from "../api/client";
import type { SearchResult } from "../types/overlay";

/**
 * 品种搜索 hook（防抖 400ms）
 */
export function useSearch(query: string) {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (timerRef.current) clearTimeout(timerRef.current);

    if (!query.trim()) {
      setResults([]);
      return;
    }

    setLoading(true);
    timerRef.current = setTimeout(async () => {
      try {
        const data = await searchSymbols(query);
        setResults(Array.isArray(data) ? data : []);
      } catch {
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 400);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [query]);

  return { results, loading };
}

import { useState, useEffect, useCallback } from "react";
import { FilterSettings, DEFAULT_FILTERS } from "@/types";

const STORAGE_KEY = "nord_scanner_filters_v2";
const LEGACY_STORAGE_KEY = "nord_tester_filters_v2";

export function usePersistentFilters() {
  const [filters, setFilters] = useState<FilterSettings>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY) || localStorage.getItem(LEGACY_STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        // Ensure default timeout is preserved if not properly set or empty
        return {
          ...DEFAULT_FILTERS,
          ...parsed,
          timeout: typeof parsed.timeout === "number" && parsed.timeout > 0 ? parsed.timeout : 30,
        };
      }
    } catch (e) {
      console.warn("Failed to read filters from localStorage:", e);
    }
    return DEFAULT_FILTERS;
  });

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(filters));
    } catch (e) {
      console.warn("Failed to persist filters to localStorage:", e);
    }
  }, [filters]);

  const updateFilter = useCallback(
    <K extends keyof FilterSettings>(key: K, value: FilterSettings[K]) => {
      setFilters((prev) => ({
        ...prev,
        [key]: value,
      }));
    },
    []
  );

  const resetFilters = useCallback(() => {
    setFilters(DEFAULT_FILTERS);
  }, []);

  return {
    filters,
    setFilters,
    updateFilter,
    resetFilters,
  };
}

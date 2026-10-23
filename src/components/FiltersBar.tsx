import React, { useState } from "react";
import {
  Filter,
  RotateCcw,
  Search,
  Globe,
  Network,
  Sliders,
  ChevronDown,
  ChevronUp,
  Cpu,
  Clock,
  Shield,
  Layers,
} from "lucide-react";
import { Card, CardContent } from "./ui/card";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Badge } from "./ui/badge";
import { FilterSettings, ServerInfo } from "@/types";

interface FiltersBarProps {
  filters: FilterSettings;
  servers: ServerInfo[];
  onUpdateFilter: <K extends keyof FilterSettings>(key: K, value: FilterSettings[K]) => void;
  onResetFilters: () => void;
}

const COMMON_PORTS = [443, 80, 1194, 51820];

const SERVER_TYPES = [
  { id: "ALL", label: "All Protocols" },
  { id: "obfuscated", label: "Obfuscated / XOR" },
  { id: "nordwhisper", label: "NordWhisper" },
  { id: "p2p", label: "P2P" },
  { id: "double", label: "Double VPN" },
  { id: "onion", label: "Onion Over VPN" },
  { id: "dedicated", label: "Dedicated IP" },
];

export const FiltersBar: React.FC<FiltersBarProps> = ({
  filters,
  servers,
  onUpdateFilter,
  onResetFilters,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);

  // Extract unique countries from server list with server count
  const countryCounts = React.useMemo(() => {
    const map = new Map<string, { name: string; count: number }>();
    for (const s of servers) {
      const code = s.country_code.toUpperCase();
      if (!map.has(code)) {
        map.set(code, { name: s.country_name, count: 0 });
      }
      map.get(code)!.count += 1;
    }
    return Array.from(map.entries())
      .map(([code, data]) => ({ code, name: data.name, count: data.count }))
      .sort((a, b) => b.count - a.count);
  }, [servers]);

  return (
    <Card className="border-slate-800/80 bg-slate-900/60 shadow-xl">
      <CardContent className="p-4 space-y-3">
        {/* Primary Filter Row: Search, Country, Port, Reset */}
        <div className="flex flex-col gap-2.5 sm:flex-row sm:items-center sm:justify-between">
          {/* Search Bar */}
          <div className="relative flex-1 min-w-[200px]">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input
              type="text"
              placeholder="Search by server name, city, or IP (e.g. us101, Frankfurt, 185.)..."
              value={filters.searchQuery}
              onChange={(e) => onUpdateFilter("searchQuery", e.target.value)}
              className="pl-9 bg-slate-950/60 border-slate-700/80 text-sm"
            />
            {filters.searchQuery && (
              <button
                onClick={() => onUpdateFilter("searchQuery", "")}
                className="absolute right-3 top-2.5 text-xs text-slate-400 hover:text-white"
              >
                Clear
              </button>
            )}
          </div>

          {/* Country Dropdown */}
          <div className="flex items-center gap-2">
            <div className="relative min-w-[160px]">
              <Globe className="absolute left-2.5 top-2.5 h-4 w-4 text-slate-400 pointer-events-none" />
              <select
                aria-label="Country Filter"
                value={filters.country}
                onChange={(e) => onUpdateFilter("country", e.target.value)}
                className="h-9 w-full appearance-none rounded-lg border border-slate-700/80 bg-slate-950/70 pl-8 pr-7 text-xs text-slate-200 focus:border-cyan-500 focus:outline-none"
              >
                <option value="ALL">All Countries ({servers.length})</option>
                {countryCounts.map((c) => (
                  <option key={c.code} value={c.code}>
                    {c.code} - {c.name} ({c.count})
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-2 top-2.5 h-4 w-4 text-slate-400 pointer-events-none" />
            </div>

            {/* Expand / Advanced settings toggle */}
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsExpanded(!isExpanded)}
              className="text-xs text-slate-300 hover:text-white border-slate-700/80"
            >
              <Sliders className="mr-1.5 h-3.5 w-3.5 text-cyan-400" />
              Filters
              {isExpanded ? (
                <ChevronUp className="ml-1 h-3.5 w-3.5" />
              ) : (
                <ChevronDown className="ml-1 h-3.5 w-3.5" />
              )}
            </Button>

            {/* Reset Button (Mandatory Requirement) */}
            <Button
              variant="ghost"
              size="sm"
              onClick={onResetFilters}
              className="text-xs text-rose-400 hover:bg-rose-500/10 hover:text-rose-300"
              title="Reset all filters to defaults (Timeout: 30s, Port: 443, etc.)"
            >
              <RotateCcw className="mr-1 h-3.5 w-3.5" />
              Reset
            </Button>
          </div>
        </div>

        {/* Quick Type Selection Pills */}
        <div className="flex flex-wrap items-center gap-1.5 pt-1">
          <span className="text-[11px] font-medium text-slate-400 mr-1 flex items-center gap-1">
            <Layers className="h-3 w-3" /> Type:
          </span>
          {SERVER_TYPES.map((t) => {
            const isSelected = filters.serverType === t.id;
            return (
              <button
                key={t.id}
                onClick={() => onUpdateFilter("serverType", t.id)}
                className={`rounded-full px-2.5 py-0.5 text-xs font-medium transition-all ${
                  isSelected
                    ? "bg-cyan-500 text-white shadow-sm shadow-cyan-500/30"
                    : "bg-slate-800/80 text-slate-400 hover:bg-slate-700 hover:text-slate-200"
                }`}
              >
                {t.label}
              </button>
            );
          })}
        </div>

        {/* Collapsible Advanced Filters Section */}
        {isExpanded && (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 pt-3 border-t border-slate-800/80">
            {/* Target Port */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                <Network className="h-3.5 w-3.5 text-cyan-400" />
                Target Port
              </label>
              <div className="flex items-center gap-1.5">
                {COMMON_PORTS.map((p) => (
                  <button
                    key={p}
                    onClick={() => onUpdateFilter("port", p)}
                    className={`px-2 py-1 text-xs rounded font-mono ${
                      filters.port === p
                        ? "bg-cyan-500/20 text-cyan-300 border border-cyan-500/40"
                        : "bg-slate-800/60 text-slate-400 hover:text-white"
                    }`}
                  >
                    {p}
                  </button>
                ))}
                <Input
                  type="number"
                  min={1}
                  max={65535}
                  value={filters.port}
                  onChange={(e) =>
                    onUpdateFilter("port", parseInt(e.target.value) || 443)
                  }
                  className="w-16 h-7 text-xs font-mono text-center p-1"
                />
              </div>
            </div>

            {/* Socket Timeout (Default 30 seconds!) */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                  <Clock className="h-3.5 w-3.5 text-amber-400" />
                  Timeout
                </label>
                <span className="text-xs font-mono text-cyan-400">
                  {filters.timeout}s {filters.timeout === 30 && "(Default)"}
                </span>
              </div>
              <input
                type="range"
                min={1}
                max={60}
                value={filters.timeout}
                onChange={(e) => onUpdateFilter("timeout", parseFloat(e.target.value))}
                className="w-full h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-cyan-400"
              />
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>1s</span>
                <span>30s (Default)</span>
                <span>60s</span>
              </div>
            </div>

            {/* Concurrency Workers */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                  <Cpu className="h-3.5 w-3.5 text-purple-400" />
                  Concurrency
                </label>
                <span className="text-xs font-mono text-purple-400">
                  {filters.concurrency} workers
                </span>
              </div>
              <input
                type="range"
                min={10}
                max={1000}
                step={10}
                value={filters.concurrency}
                onChange={(e) =>
                  onUpdateFilter("concurrency", parseInt(e.target.value) || 200)
                }
                className="w-full h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-purple-400"
              />
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>10</span>
                <span>200</span>
                <span>1000</span>
              </div>
            </div>

            {/* Mode & Sort Selection */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
                <Shield className="h-3.5 w-3.5 text-emerald-400" />
                Connection Mode & Sort
              </label>
              <div className="flex items-center gap-2">
                <select
                  aria-label="Connection Mode"
                  value={filters.mode}
                  onChange={(e) =>
                    onUpdateFilter("mode", e.target.value as "tcp" | "tls")
                  }
                  className="h-7 rounded border border-slate-700 bg-slate-950 px-2 text-xs text-slate-200"
                >
                  <option value="tcp">TCP (Fastest)</option>
                  <option value="tls">TLS Handshake</option>
                </select>

                <select
                  aria-label="Sort By"
                  value={filters.sortBy}
                  onChange={(e) =>
                    onUpdateFilter("sortBy", e.target.value as any)
                  }
                  className="h-7 flex-1 rounded border border-slate-700 bg-slate-950 px-2 text-xs text-slate-200"
                >
                  <option value="latency_asc">Ping: Low to High</option>
                  <option value="latency_desc">Ping: High to Low</option>
                  <option value="load_asc">Load: Low to High</option>
                  <option value="name_asc">Server Name</option>
                  <option value="country_asc">Country Name</option>
                </select>
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
};

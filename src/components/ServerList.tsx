import React, { useState, useMemo } from "react";
import {
  Server,
  Copy,
  Check,
  ChevronLeft,
  ChevronRight,
  Wifi,
  ExternalLink,
  Shield,
  Clock,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { ServerInfo, TestResult } from "@/types";

interface ServerListProps {
  servers: ServerInfo[];
  resultsMap: Map<number, TestResult>;
}

const ITEMS_PER_PAGE = 50;

export const ServerList: React.FC<ServerListProps> = ({
  servers,
  resultsMap,
}) => {
  const [currentPage, setCurrentPage] = useState(1);
  const [copiedId, setCopiedId] = useState<number | null>(null);

  const totalPages = Math.max(1, Math.ceil(servers.length / ITEMS_PER_PAGE));

  // Reset to page 1 if servers list changes length significantly
  React.useEffect(() => {
    setCurrentPage(1);
  }, [servers.length]);

  const pagedServers = useMemo(() => {
    const start = (currentPage - 1) * ITEMS_PER_PAGE;
    return servers.slice(start, start + ITEMS_PER_PAGE);
  }, [servers, currentPage]);

  const copyToClipboard = (text: string, id: number) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const getLatencyColor = (ms: number | null | undefined) => {
    if (ms === null || ms === undefined) return "text-slate-500";
    if (ms < 50) return "text-emerald-400 font-bold";
    if (ms < 100) return "text-cyan-400 font-semibold";
    if (ms < 200) return "text-amber-400 font-medium";
    return "text-rose-400 font-medium";
  };

  return (
    <Card className="border-slate-800/80 bg-slate-900/60 shadow-2xl overflow-hidden">
      <CardHeader className="py-3 px-4 sm:px-6 border-b border-slate-800/60 flex flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <Server className="h-4 w-4 text-cyan-400" />
          <CardTitle className="text-sm font-semibold text-white">
            Servers ({servers.length.toLocaleString()})
          </CardTitle>
        </div>

        {/* Pagination Controls */}
        {totalPages > 1 && (
          <div className="flex items-center gap-2 text-xs">
            <span className="text-slate-400">
              Page <strong className="text-slate-200">{currentPage}</strong> of{" "}
              {totalPages}
            </span>
            <div className="flex items-center gap-1">
              <Button
                variant="outline"
                size="xs"
                disabled={currentPage === 1}
                onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
              >
                <ChevronLeft className="h-3.5 w-3.5" />
              </Button>
              <Button
                variant="outline"
                size="xs"
                disabled={currentPage === totalPages}
                onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
              >
                <ChevronRight className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        )}
      </CardHeader>

      <CardContent className="p-0">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-slate-950/60 text-slate-400 border-b border-slate-800/80">
              <tr>
                <th className="py-2.5 px-3 sm:px-4 font-medium">Status</th>
                <th className="py-2.5 px-3 font-medium">Server</th>
                <th className="py-2.5 px-3 font-medium">Location</th>
                <th className="py-2.5 px-3 font-medium">Station IP</th>
                <th className="py-2.5 px-3 font-medium">Latency</th>
                <th className="py-2.5 px-3 font-medium">Load</th>
                <th className="py-2.5 px-3 sm:px-4 font-medium">Protocols / Tags</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/40 font-normal text-slate-200">
              {pagedServers.length === 0 ? (
                <tr>
                  <td colSpan={7} className="py-8 text-center text-slate-500">
                    No servers match the selected criteria. Try adjusting your filters.
                  </td>
                </tr>
              ) : (
                pagedServers.map((s) => {
                  const test = resultsMap.get(s.id);
                  const isOnline = test?.status === "ONLINE";
                  const isTimeout = test?.status === "TIMEOUT";
                  const isFailed = test && !isOnline && !isTimeout;

                  return (
                    <tr
                      key={s.id}
                      className="hover:bg-slate-800/30 transition-colors group"
                    >
                      {/* Status */}
                      <td className="py-2.5 px-3 sm:px-4">
                        {test ? (
                          isOnline ? (
                            <Badge variant="success" className="text-[10px] px-1.5 py-0 h-4">
                              ONLINE
                            </Badge>
                          ) : isTimeout ? (
                            <Badge variant="destructive" className="text-[10px] px-1.5 py-0 h-4">
                              TIMEOUT
                            </Badge>
                          ) : (
                            <Badge variant="warning" className="text-[10px] px-1.5 py-0 h-4">
                              {test.status}
                            </Badge>
                          )
                        ) : (
                          <Badge variant="outline" className="text-[10px] px-1.5 py-0 h-4 text-slate-500">
                            UNTESTED
                          </Badge>
                        )}
                      </td>

                      {/* Server Name & Hostname */}
                      <td className="py-2.5 px-3">
                        <div className="font-medium text-white truncate max-w-[150px] sm:max-w-[200px]">
                          {s.name}
                        </div>
                        <div className="text-[11px] font-mono text-slate-500 truncate max-w-[150px] sm:max-w-[200px]">
                          {s.hostname}
                        </div>
                      </td>

                      {/* Location */}
                      <td className="py-2.5 px-3 whitespace-nowrap">
                        <div className="flex items-center gap-1.5">
                          <span className="font-semibold text-slate-300">
                            {s.country_code}
                          </span>
                          <span className="text-slate-400 truncate max-w-[120px]">
                            {s.city || s.country_name}
                          </span>
                        </div>
                      </td>

                      {/* Station IP */}
                      <td className="py-2.5 px-3 font-mono text-slate-300">
                        <div className="flex items-center gap-1.5">
                          <span>{s.station}</span>
                          <button
                            onClick={() => copyToClipboard(s.station, s.id)}
                            className="text-slate-500 hover:text-white p-0.5 rounded transition-colors"
                            title="Copy IP"
                          >
                            {copiedId === s.id ? (
                              <Check className="h-3 w-3 text-emerald-400" />
                            ) : (
                              <Copy className="h-3 w-3" />
                            )}
                          </button>
                        </div>
                      </td>

                      {/* Latency */}
                      <td className="py-2.5 px-3 font-mono whitespace-nowrap">
                        {test?.latency_ms !== null && test?.latency_ms !== undefined ? (
                          <span className={getLatencyColor(test.latency_ms)}>
                            {test.latency_ms} ms
                          </span>
                        ) : isTimeout ? (
                          <span className="text-rose-500/80 text-[11px]">Timeout</span>
                        ) : test?.error_msg ? (
                          <span className="text-slate-500 text-[11px]" title={test.error_msg}>
                            Error
                          </span>
                        ) : (
                          <span className="text-slate-600">--</span>
                        )}
                      </td>

                      {/* Load */}
                      <td className="py-2.5 px-3 whitespace-nowrap">
                        <div className="flex items-center gap-2">
                          <div className="w-12 h-1.5 rounded-full bg-slate-800 overflow-hidden">
                            <div
                              className={`h-full rounded-full ${
                                s.load < 40
                                  ? "bg-emerald-500"
                                  : s.load < 75
                                  ? "bg-amber-500"
                                  : "bg-rose-500"
                              }`}
                              style={{ width: `${s.load}%` }}
                            />
                          </div>
                          <span className="font-mono text-[11px] text-slate-400">
                            {s.load}%
                          </span>
                        </div>
                      </td>

                      {/* Protocols / Tags */}
                      <td className="py-2.5 px-3 sm:px-4">
                        <div className="flex flex-wrap gap-1 max-w-[260px]">
                          {s.technologies.some(
                            (t) => t.includes("xor") || t.includes("obfusc")
                          ) && (
                            <span className="rounded bg-cyan-950/60 text-cyan-300 border border-cyan-800/40 px-1.5 py-0.2 text-[10px]">
                              Obfuscated
                            </span>
                          )}
                          {s.technologies.some((t) => t.includes("whisper")) && (
                            <span className="rounded bg-purple-950/60 text-purple-300 border border-purple-800/40 px-1.5 py-0.2 text-[10px]">
                              NordWhisper
                            </span>
                          )}
                          {s.groups.some((g) => g.includes("p2p")) && (
                            <span className="rounded bg-emerald-950/60 text-emerald-300 border border-emerald-800/40 px-1.5 py-0.2 text-[10px]">
                              P2P
                            </span>
                          )}
                          {s.groups.some((g) => g.includes("double")) && (
                            <span className="rounded bg-amber-950/60 text-amber-300 border border-amber-800/40 px-1.5 py-0.2 text-[10px]">
                              DoubleVPN
                            </span>
                          )}
                          {s.groups.some((g) => g.includes("onion")) && (
                            <span className="rounded bg-rose-950/60 text-rose-300 border border-rose-800/40 px-1.5 py-0.2 text-[10px]">
                              Onion
                            </span>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
};

import React from "react";
import { Trophy, Copy, Check, Zap, Server } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { Badge } from "./ui/badge";
import { TestResult } from "@/types";

interface LeaderboardProps {
  results: TestResult[];
}

export const Leaderboard: React.FC<LeaderboardProps> = ({ results }) => {
  const [copiedId, setCopiedId] = React.useState<number | null>(null);

  // Filter online results with valid latency, sorted by latency ascending
  const top10 = React.useMemo(() => {
    return results
      .filter((r) => r.status === "ONLINE" && r.latency_ms !== null && r.latency_ms > 0)
      .sort((a, b) => (a.latency_ms ?? 99999) - (b.latency_ms ?? 99999))
      .slice(0, 10);
  }, [results]);

  const copyToClipboard = (station: string, id: number) => {
    navigator.clipboard.writeText(station);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  if (top10.length === 0) {
    return null;
  }

  return (
    <Card className="border-slate-800/80 bg-slate-900/60 shadow-xl overflow-hidden">
      <CardHeader className="py-3 px-4 border-b border-slate-800/60 flex flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <Trophy className="h-4 w-4 text-amber-400" />
          <CardTitle className="text-sm font-semibold text-white">
            Top 10 Fastest Online Servers
          </CardTitle>
        </div>
        <span className="text-xs text-slate-400">Lowest Socket Round-Trip</span>
      </CardHeader>
      <CardContent className="p-0">
        <div className="grid grid-cols-1 divide-y divide-slate-800/50 sm:grid-cols-2 sm:divide-y-0 sm:gap-2 sm:p-2 sm:divide-x-0">
          {top10.map((server, index) => {
            const rankColors = [
              "text-amber-400 font-bold", // #1 Gold
              "text-slate-300 font-bold", // #2 Silver
              "text-amber-600 font-bold", // #3 Bronze
            ];
            const rankClass = rankColors[index] ?? "text-slate-500 font-medium";

            return (
              <div
                key={server.server_id}
                className="flex items-center justify-between p-2.5 rounded-lg hover:bg-slate-800/40 transition-colors"
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <span className={`w-5 text-center font-mono text-xs ${rankClass}`}>
                    #{index + 1}
                  </span>
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <span className="text-xs font-semibold text-slate-100 truncate">
                        {server.name}
                      </span>
                      <Badge variant="outline" className="text-[10px] px-1.5 py-0 h-4">
                        {server.country_code}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2 text-[11px] text-slate-400">
                      <span>{server.city || server.country_name}</span>
                      <span>•</span>
                      <span className="font-mono text-slate-500">{server.station}</span>
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-3 shrink-0 ml-2">
                  <div className="text-right">
                    <div className="text-xs font-mono font-bold text-emerald-400 flex items-center justify-end gap-1">
                      <Zap className="h-3 w-3" />
                      {server.latency_ms} ms
                    </div>
                    <div className="text-[10px] text-slate-400">
                      Load: {server.load}%
                    </div>
                  </div>

                  <button
                    onClick={() => copyToClipboard(server.station, server.server_id)}
                    className="p-1 rounded text-slate-400 hover:text-white hover:bg-slate-700/60"
                    title="Copy IP Address"
                  >
                    {copiedId === server.server_id ? (
                      <Check className="h-3.5 w-3.5 text-emerald-400" />
                    ) : (
                      <Copy className="h-3.5 w-3.5" />
                    )}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
};

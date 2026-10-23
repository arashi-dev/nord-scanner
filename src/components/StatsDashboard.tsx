import React from "react";
import {
  Play,
  Square,
  Pause,
  Activity,
  Zap,
  CheckCircle2,
  XCircle,
  Timer,
  Gauge,
} from "lucide-react";
import { Card, CardContent } from "./ui/card";
import { Button } from "./ui/button";
import { Progress } from "./ui/progress";
import { ScanProgress } from "@/types";

interface StatsDashboardProps {
  progress: ScanProgress | null;
  isScanning: boolean;
  isPaused: boolean;
  totalFiltered: number;
  onStartScan: () => void;
  onStopScan: () => void;
  onTogglePause: () => void;
}

export const StatsDashboard: React.FC<StatsDashboardProps> = ({
  progress,
  isScanning,
  isPaused,
  totalFiltered,
  onStartScan,
  onStopScan,
  onTogglePause,
}) => {
  const scanned = progress?.scanned ?? 0;
  const total = progress?.total || totalFiltered || 0;
  const percent = total > 0 ? Math.round((scanned / total) * 100) : 0;
  const online = progress?.online ?? 0;
  const offline = progress?.offline ?? 0;
  const avgPing = progress?.avg_latency_ms ?? 0;
  const minPing = progress?.min_latency_ms ?? 0;
  const maxPing = progress?.max_latency_ms ?? 0;
  const speed = progress?.speed_eps ?? 0;
  const elapsed = progress?.elapsed_secs ?? 0;

  // Calculate ETA
  const remaining = total - scanned;
  const etaSecs = speed > 0 && remaining > 0 ? Math.round(remaining / speed) : 0;

  return (
    <Card className="border-slate-800/80 bg-slate-900/60 shadow-2xl">
      <CardContent className="p-4 sm:p-6 space-y-4">
        {/* Controls and Header Row */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-slate-800 text-cyan-400 border border-slate-700/60">
              <Activity className={`h-5 w-5 ${isScanning && !isPaused ? "animate-pulse text-cyan-400" : ""}`} />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-base font-semibold text-white">Live Scanner Dashboard</h2>
                {isScanning && (
                  <span className="flex h-2 w-2 rounded-full bg-cyan-400 animate-ping" />
                )}
              </div>
              <p className="text-xs text-slate-400">
                {isScanning
                  ? isPaused
                    ? "Scan paused"
                    : `Benchmarking socket latency (${speed} tests/s)...`
                  : scanned > 0
                  ? "Scan completed"
                  : "Ready to scan servers"}
              </p>
            </div>
          </div>

          {/* Action Buttons */}
          <div className="flex items-center gap-2">
            {!isScanning ? (
              <Button
                variant="default"
                size="default"
                onClick={onStartScan}
                className="bg-gradient-to-r from-cyan-500 to-blue-600 hover:from-cyan-400 hover:to-blue-500 text-white font-semibold shadow-lg shadow-cyan-500/20"
              >
                <Play className="mr-1.5 h-4 w-4 fill-current" />
                Scan All Servers
              </Button>
            ) : (
              <>
                <Button
                  variant="secondary"
                  size="default"
                  onClick={onTogglePause}
                  className="bg-slate-800 hover:bg-slate-700 text-slate-200"
                >
                  {isPaused ? (
                    <>
                      <Play className="mr-1.5 h-4 w-4 fill-current text-emerald-400" />
                      Resume
                    </>
                  ) : (
                    <>
                      <Pause className="mr-1.5 h-4 w-4 fill-current text-amber-400" />
                      Pause
                    </>
                  )}
                </Button>
                <Button
                  variant="destructive"
                  size="default"
                  onClick={onStopScan}
                  className="shadow-md shadow-rose-600/20"
                >
                  <Square className="mr-1.5 h-4 w-4 fill-current" />
                  Stop Scan
                </Button>
              </>
            )}
          </div>
        </div>

        {/* Progress Bar & Percent */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between text-xs text-slate-400">
            <span>
              Progress: <strong className="text-slate-200">{scanned.toLocaleString()}</strong> /{" "}
              {total.toLocaleString()} servers ({percent}%)
            </span>
            <span>
              Elapsed: <strong className="text-slate-200">{elapsed}s</strong>
              {etaSecs > 0 && ` | ETA: ~${etaSecs}s`}
            </span>
          </div>
          <Progress value={scanned} max={total || 1} className="h-2.5" />
        </div>

        {/* Metrics Grid */}
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 lg:grid-cols-6 pt-1">
          {/* Online */}
          <div className="rounded-lg border border-emerald-950/40 bg-emerald-950/20 p-2.5 sm:p-3">
            <div className="flex items-center gap-1.5 text-xs font-medium text-emerald-400">
              <CheckCircle2 className="h-3.5 w-3.5" />
              Online
            </div>
            <div className="mt-1 text-lg font-bold text-emerald-300">
              {online.toLocaleString()}
            </div>
            <div className="text-[10px] text-emerald-500/80">
              {scanned > 0 ? Math.round((online / scanned) * 100) : 0}% success
            </div>
          </div>

          {/* Timeout / Refused */}
          <div className="rounded-lg border border-rose-950/40 bg-rose-950/20 p-2.5 sm:p-3">
            <div className="flex items-center gap-1.5 text-xs font-medium text-rose-400">
              <XCircle className="h-3.5 w-3.5" />
              Timeout/Failed
            </div>
            <div className="mt-1 text-lg font-bold text-rose-300">
              {offline.toLocaleString()}
            </div>
            <div className="text-[10px] text-rose-500/80">
              {scanned > 0 ? Math.round((offline / scanned) * 100) : 0}% failure
            </div>
          </div>

          {/* Avg Ping */}
          <div className="rounded-lg border border-slate-800 bg-slate-900/60 p-2.5 sm:p-3">
            <div className="flex items-center gap-1.5 text-xs font-medium text-slate-400">
              <Gauge className="h-3.5 w-3.5 text-cyan-400" />
              Avg Latency
            </div>
            <div className="mt-1 text-lg font-bold text-cyan-300">
              {online > 0 ? `${avgPing} ms` : "--"}
            </div>
            <div className="text-[10px] text-slate-500">round-trip socket</div>
          </div>

          {/* Min Ping */}
          <div className="rounded-lg border border-slate-800 bg-slate-900/60 p-2.5 sm:p-3">
            <div className="flex items-center gap-1.5 text-xs font-medium text-slate-400">
              <Zap className="h-3.5 w-3.5 text-emerald-400" />
              Fastest Ping
            </div>
            <div className="mt-1 text-lg font-bold text-emerald-300">
              {online > 0 && minPing > 0 ? `${minPing} ms` : "--"}
            </div>
            <div className="text-[10px] text-slate-500">lowest response</div>
          </div>

          {/* Max Ping */}
          <div className="rounded-lg border border-slate-800 bg-slate-900/60 p-2.5 sm:p-3">
            <div className="flex items-center gap-1.5 text-xs font-medium text-slate-400">
              <Timer className="h-3.5 w-3.5 text-amber-400" />
              Max Latency
            </div>
            <div className="mt-1 text-lg font-bold text-amber-300">
              {online > 0 && maxPing > 0 ? `${maxPing} ms` : "--"}
            </div>
            <div className="text-[10px] text-slate-500">highest response</div>
          </div>

          {/* Scan Speed */}
          <div className="rounded-lg border border-slate-800 bg-slate-900/60 p-2.5 sm:p-3">
            <div className="flex items-center gap-1.5 text-xs font-medium text-slate-400">
              <Activity className="h-3.5 w-3.5 text-purple-400" />
              Scan Speed
            </div>
            <div className="mt-1 text-lg font-bold text-purple-300">
              {speed > 0 ? `${speed} /s` : "--"}
            </div>
            <div className="text-[10px] text-slate-500">concurrent tests</div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};

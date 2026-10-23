import React, { useState, useEffect } from "react";
import {
  RefreshCw,
  Download,
  Trash2,
  Sparkles,
  Clock,
  CheckCircle,
} from "lucide-react";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { formatTimeAgo } from "@/lib/utils";
import {
  refreshServers,
  clearResultsCache,
  exportResultsFile,
  checkForAppUpdates,
  UpdateInfo,
} from "@/services/tauri";
import { TestResult } from "@/types";

interface HeaderProps {
  appVersion: string;
  lastRefreshedAt: number | null;
  serverCount: number;
  testResults: TestResult[];
  onServersRefreshed: (count: number, timestamp: number) => void;
  onCacheCleared: () => void;
  onUpdateFound: (update: UpdateInfo) => void;
}

export const Header: React.FC<HeaderProps> = ({
  appVersion,
  lastRefreshedAt,
  serverCount,
  testResults,
  onServersRefreshed,
  onCacheCleared,
  onUpdateFound,
}) => {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [, setTick] = useState(0);

  // Re-calculate the time-ago every 10 seconds dynamically
  useEffect(() => {
    const timer = setInterval(() => {
      setTick((t) => t + 1);
    }, 10000);
    return () => clearInterval(timer);
  }, []);

  const handleManualRefresh = async () => {
    setIsRefreshing(true);
    try {
      const res = await refreshServers();
      onServersRefreshed(res.total_servers, res.refreshed_at);
    } catch (err) {
      console.error("Refresh failed:", err);
      alert(`Failed to refresh servers: ${err}`);
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleCheckUpdate = async () => {
    setIsCheckingUpdate(true);
    setUpdateStatus(null);
    try {
      const info = await checkForAppUpdates();
      if (info.available) {
        onUpdateFound(info);
      } else {
        setUpdateStatus("Up to date");
        setTimeout(() => setUpdateStatus(null), 3500);
      }
    } catch (e) {
      console.warn("Update check failed", e);
      setUpdateStatus("Check failed");
      setTimeout(() => setUpdateStatus(null), 3000);
    } finally {
      setIsCheckingUpdate(false);
    }
  };

  const handleExport = async (format: "csv" | "json") => {
    if (testResults.length === 0) {
      alert("No test results to export yet. Run a scan first!");
      return;
    }
    try {
      const savedPath = await exportResultsFile(testResults, format);
      alert(`Exported ${testResults.length} servers to ${savedPath}`);
    } catch (err) {
      alert(`Export error: ${err}`);
    }
  };

  const handleClearCache = async () => {
    if (confirm("Clear local benchmark test cache? You will need to re-test servers.")) {
      await clearResultsCache();
      onCacheCleared();
    }
  };

  return (
    <header className="sticky top-0 z-40 w-full border-b border-slate-800/80 bg-slate-950/90 backdrop-blur-md px-4 py-3 sm:px-6">
      <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        {/* Brand & Stats info */}
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-amber-500/20 to-yellow-500/10 border border-amber-500/30 shadow-lg shadow-amber-500/15">
            <svg
              viewBox="0 0 24 24"
              className="h-6 w-6 drop-shadow-sm"
              xmlns="http://www.w3.org/2000/svg"
            >
              <defs>
                <linearGradient id="headerNordYellow" x1="0%" y1="0%" x2="0%" y2="100%">
                  <stop offset="0%" stopColor="#FBBF24" />
                  <stop offset="100%" stopColor="#F59E0B" />
                </linearGradient>
              </defs>
              <path
                fill="url(#headerNordYellow)"
                d="M2.2838 21.5414A11.9866 11.9866 0 010 14.4832C0 7.8418 5.3727 2.4586 12 2.4586c6.6279 0 12 5.3832 12 12.0246a11.9853 11.9853 0 01-2.2838 7.0582l-5.7636-9.3783-.5565.9419.5645 2.6186L12 8.9338l-2.45 4.1447.5707 2.6451-2.0764-3.5555-5.7605 9.3733z"
              />
            </svg>
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-lg font-bold tracking-tight text-white">
                Nord Scanner
              </h1>
              <Badge variant="secondary" className="font-mono text-[10px]">
                v{appVersion}
              </Badge>
              {serverCount > 0 && (
                <Badge variant="default" className="text-[10px]">
                  {serverCount.toLocaleString()} Servers
                </Badge>
              )}
            </div>
            
            {/* Last Refreshed in Time-Ago format */}
            <div className="flex items-center gap-1.5 text-xs text-slate-400 mt-0.5">
              <Clock className="h-3 w-3 text-cyan-400" />
              <span>Catalog refreshed:</span>
              <span className="font-medium text-slate-200">
                {formatTimeAgo(lastRefreshedAt)}
              </span>
            </div>
          </div>
        </div>

        {/* Global Action Buttons */}
        <div className="flex flex-wrap items-center gap-2">
          {/* Manual Refresh Server Catalog (Separate from scanning) */}
          <Button
            variant="outline"
            size="sm"
            onClick={handleManualRefresh}
            disabled={isRefreshing}
            className="border-cyan-500/30 text-cyan-400 hover:bg-cyan-500/10 hover:border-cyan-500/60"
            title="Fetch full server catalog from NordVPN API separately"
          >
            <RefreshCw
              className={`mr-1.5 h-3.5 w-3.5 ${isRefreshing ? "animate-spin" : ""}`}
            />
            {isRefreshing ? "Refreshing..." : "Refresh Servers"}
          </Button>

          {/* Auto Update Check */}
          <Button
            variant="outline"
            size="sm"
            onClick={handleCheckUpdate}
            disabled={isCheckingUpdate}
            className="text-slate-300 hover:text-white"
            title="Check for application updates"
          >
            <Sparkles
              className={`mr-1.5 h-3.5 w-3.5 ${isCheckingUpdate ? "animate-spin text-amber-400" : "text-amber-400"}`}
            />
            {isCheckingUpdate ? "Checking..." : updateStatus ? updateStatus : "Check Updates"}
          </Button>

          {/* Export Dropdown / Buttons */}
          <Button
            variant="secondary"
            size="sm"
            onClick={() => handleExport("csv")}
            className="text-slate-300"
            title="Export tested results to CSV"
          >
            <Download className="mr-1.5 h-3.5 w-3.5 text-emerald-400" />
            CSV
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => handleExport("json")}
            className="text-slate-300"
            title="Export tested results to JSON"
          >
            <Download className="mr-1.5 h-3.5 w-3.5 text-cyan-400" />
            JSON
          </Button>

          {/* Clear Cache */}
          <Button
            variant="ghost"
            size="icon"
            onClick={handleClearCache}
            className="text-slate-400 hover:text-rose-400 hover:bg-rose-500/10"
            title="Clear benchmark test results cache"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </header>
  );
};

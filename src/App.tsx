import React, { useState, useEffect, useMemo, useCallback } from "react";
import { Header } from "./components/Header";
import { StatsDashboard } from "./components/StatsDashboard";
import { FiltersBar } from "./components/FiltersBar";
import { Leaderboard } from "./components/Leaderboard";
import { ServerList } from "./components/ServerList";
import { UpdaterModal } from "./components/UpdaterModal";
import { usePersistentFilters } from "./hooks/usePersistentFilters";
import {
  getMetadata,
  loadServers,
  getCachedResults,
  startScan,
  stopScan,
  togglePauseScan,
  getAppVersion,
  checkForAppUpdates,
  onScanResult,
  onScanProgress,
  onScanFinished,
  UpdateInfo,
} from "./services/tauri";
import { ServerInfo, TestResult, ScanProgress } from "./types";

export function App() {
  const { filters, updateFilter, resetFilters } = usePersistentFilters();

  const [allServers, setAllServers] = useState<ServerInfo[]>([]);
  const [resultsMap, setResultsMap] = useState<Map<number, TestResult>>(new Map());
  const [lastRefreshedAt, setLastRefreshedAt] = useState<number | null>(null);
  const [appVersion, setAppVersion] = useState("0.1.0");

  const [isScanning, setIsScanning] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);

  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);

  // Load initial app data
  useEffect(() => {
    async function init() {
      try {
        const ver = await getAppVersion();
        setAppVersion(ver);

        // Check metadata & load servers
        const meta = await getMetadata();
        if (meta.last_modified) {
          setLastRefreshedAt(meta.last_modified);
        }

        const servers = await loadServers();
        setAllServers(servers);

        // Load cached benchmark results
        const cached = await getCachedResults();
        if (cached && cached.length > 0) {
          const map = new Map<number, TestResult>();
          for (const r of cached) {
            map.set(r.server_id, r);
          }
          setResultsMap(map);
        }

        // Background update check on startup
        const update = await checkForAppUpdates();
        if (update.available) {
          setUpdateInfo(update);
        }
      } catch (err) {
        console.error("Initialization error:", err);
      }
    }
    init();
  }, []);

  // Listen to Tauri scan events
  useEffect(() => {
    let unlistenResult: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    let unlistenFinished: (() => void) | undefined;

    async function setupListeners() {
      unlistenResult = await onScanResult((result) => {
        setResultsMap((prev) => {
          const next = new Map(prev);
          next.set(result.server_id, result);
          return next;
        });
      });

      unlistenProgress = await onScanProgress((progress) => {
        setScanProgress(progress);
        setIsScanning(progress.is_running);
        setIsPaused(progress.is_paused);
      });

      unlistenFinished = await onScanFinished(() => {
        setIsScanning(false);
        setIsPaused(false);
      });
    }

    setupListeners();

    return () => {
      unlistenResult?.();
      unlistenProgress?.();
      unlistenFinished?.();
    };
  }, []);

  // Filter and sort servers
  const filteredServers = useMemo(() => {
    return allServers
      .filter((server) => {
        // Country filter
        if (
          filters.country !== "ALL" &&
          server.country_code.toUpperCase() !== filters.country.toUpperCase()
        ) {
          return false;
        }

        // Protocol / Type filter
        if (filters.serverType !== "ALL") {
          const type = filters.serverType.toLowerCase();
          const matchTech = server.technologies.some((t) =>
            t.toLowerCase().includes(type)
          );
          const matchGroup = server.groups.some((g) =>
            g.toLowerCase().includes(type)
          );
          if (!matchTech && !matchGroup) {
            return false;
          }
        }

        // Status filter
        if (filters.statusFilter !== "ALL") {
          const res = resultsMap.get(server.id);
          if (filters.statusFilter === "ONLINE" && res?.status !== "ONLINE") {
            return false;
          }
          if (filters.statusFilter === "TIMEOUT" && res?.status !== "TIMEOUT") {
            return false;
          }
          if (
            filters.statusFilter === "ERROR" &&
            (!res || res.status === "ONLINE" || res.status === "TIMEOUT")
          ) {
            return false;
          }
        }

        // Max Latency slider
        if (filters.maxLatency > 0) {
          const res = resultsMap.get(server.id);
          if (!res || !res.latency_ms || res.latency_ms > filters.maxLatency) {
            return false;
          }
        }

        // Search text
        if (filters.searchQuery.trim()) {
          const q = filters.searchQuery.trim().toLowerCase();
          const matchName = server.name.toLowerCase().includes(q);
          const matchHost = server.hostname.toLowerCase().includes(q);
          const matchCity = server.city.toLowerCase().includes(q);
          const matchIP = server.station.includes(q);
          if (!matchName && !matchHost && !matchCity && !matchIP) {
            return false;
          }
        }

        return true;
      })
      .sort((a, b) => {
        const resA = resultsMap.get(a.id);
        const resB = resultsMap.get(b.id);

        switch (filters.sortBy) {
          case "latency_asc": {
            const latA = resA?.latency_ms ?? 999999;
            const latB = resB?.latency_ms ?? 999999;
            return latA - latB;
          }
          case "latency_desc": {
            const latA = resA?.latency_ms ?? -1;
            const latB = resB?.latency_ms ?? -1;
            return latB - latA;
          }
          case "load_asc":
            return a.load - b.load;
          case "name_asc":
            return a.name.localeCompare(b.name);
          case "country_asc":
            return a.country_name.localeCompare(b.country_name);
          default:
            return 0;
        }
      });
  }, [allServers, resultsMap, filters]);

  // Scan Handlers
  const handleStartScan = useCallback(async () => {
    setIsScanning(true);
    setIsPaused(false);
    try {
      await startScan({
        port: filters.port,
        timeout_secs: filters.timeout, // Default timeout 30s
        concurrency: filters.concurrency,
        mode: filters.mode,
        country_filter:
          filters.country !== "ALL" ? [filters.country] : undefined,
        feature_filter:
          filters.serverType !== "ALL" ? [filters.serverType] : undefined,
      });
    } catch (err: any) {
      alert(`Scan failed to start: ${err?.message || err}`);
      setIsScanning(false);
    }
  }, [filters]);

  const handleStopScan = useCallback(async () => {
    await stopScan();
    setIsScanning(false);
    setIsPaused(false);
  }, []);

  const handleTogglePause = useCallback(async () => {
    const nextPaused = await togglePauseScan();
    setIsPaused(nextPaused);
  }, []);

  const handleServersRefreshed = (count: number, timestamp: number) => {
    setLastRefreshedAt(timestamp);
    loadServers().then((servers) => setAllServers(servers));
  };

  const handleCacheCleared = () => {
    setResultsMap(new Map());
    setScanProgress(null);
  };

  return (
    <div className="min-h-screen bg-[#080d1a] text-slate-100 flex flex-col font-sans">
      {/* Header with App Title, Last Refreshed Time-Ago, and Updates */}
      <Header
        appVersion={appVersion}
        lastRefreshedAt={lastRefreshedAt}
        serverCount={allServers.length}
        testResults={Array.from(resultsMap.values())}
        onServersRefreshed={handleServersRefreshed}
        onCacheCleared={handleCacheCleared}
        onUpdateFound={(info) => setUpdateInfo(info)}
      />

      {/* Main Container */}
      <main className="flex-1 p-3 sm:p-5 max-w-7xl mx-auto w-full space-y-4">
        {/* Live Metrics & Progress Dashboard */}
        <StatsDashboard
          progress={scanProgress}
          isScanning={isScanning}
          isPaused={isPaused}
          totalFiltered={filteredServers.length}
          onStartScan={handleStartScan}
          onStopScan={handleStopScan}
          onTogglePause={handleTogglePause}
        />

        {/* Top 10 Fastest Servers Leaderboard */}
        <Leaderboard results={Array.from(resultsMap.values())} />

        {/* Persistent Filters Bar with Reset Button */}
        <FiltersBar
          filters={filters}
          servers={allServers}
          onUpdateFilter={updateFilter}
          onResetFilters={resetFilters}
        />

        {/* Paginated Server List Table */}
        <ServerList
          servers={filteredServers}
          resultsMap={resultsMap}
        />
      </main>

      {/* Auto-updater Modal Dialog */}
      <UpdaterModal
        updateInfo={updateInfo}
        onClose={() => setUpdateInfo(null)}
      />
    </div>
  );
}

export default App;

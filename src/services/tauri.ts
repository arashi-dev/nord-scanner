import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import {
  ServerInfo,
  TestResult,
  MetadataInfo,
  RefreshStatus,
  ScanProgress,
  FilterSettings,
} from "@/types";

export const isTauri = (): boolean => {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
};

// -----------------------------------------------------------------------------
// Tauri Backend Invocations
// -----------------------------------------------------------------------------

export async function getMetadata(): Promise<MetadataInfo> {
  if (!isTauri()) {
    return {
      exists: true,
      count: 7200,
      last_modified: Math.floor(Date.now() / 1000) - 300,
      age_seconds: 300,
    };
  }
  return await invoke<MetadataInfo>("get_metadata");
}

export async function refreshServers(): Promise<RefreshStatus> {
  if (!isTauri()) {
    return {
      total_servers: 7248,
      refreshed_at: Math.floor(Date.now() / 1000),
      age_seconds: 0,
    };
  }
  return await invoke<RefreshStatus>("refresh_servers");
}

export async function loadServers(): Promise<ServerInfo[]> {
  if (!isTauri()) {
    // Mock servers for browser preview
    return [
      {
        id: 1001,
        name: "Germany #1001",
        hostname: "de1001.nordvpn.com",
        station: "185.123.45.67",
        country_name: "Germany",
        country_code: "DE",
        city: "Frankfurt",
        load: 24,
        technologies: ["openvpn_xor", "nordwhisper"],
        groups: ["legacy_p2p"],
      },
      {
        id: 2002,
        name: "United States #2002",
        hostname: "us2002.nordvpn.com",
        station: "192.241.12.89",
        country_name: "United States",
        country_code: "US",
        city: "New York",
        load: 45,
        technologies: ["nordwhisper"],
        groups: ["legacy_standard"],
      },
    ];
  }
  return await invoke<ServerInfo[]>("load_servers");
}

export async function getCachedResults(ttlSecs?: number): Promise<TestResult[]> {
  if (!isTauri()) return [];
  return await invoke<TestResult[]>("get_cached_results", { ttlSecs });
}

export async function clearResultsCache(): Promise<void> {
  if (!isTauri()) return;
  await invoke("clear_results_cache");
}

export interface ScanOptions {
  port: number;
  timeout_secs: number;
  concurrency: number;
  mode: string;
  country_filter?: string[];
  feature_filter?: string[];
  force_retest?: boolean;
}

export async function startScan(options: ScanOptions): Promise<void> {
  if (!isTauri()) {
    console.log("Mock startScan", options);
    return;
  }
  await invoke("start_scan", { config: options });
}

export async function stopScan(): Promise<void> {
  if (!isTauri()) return;
  await invoke("stop_scan");
}

export async function togglePauseScan(): Promise<boolean> {
  if (!isTauri()) return false;
  return await invoke<boolean>("toggle_pause_scan");
}

export async function exportResultsFile(
  results: TestResult[],
  format: "csv" | "json",
  targetPath?: string
): Promise<string> {
  if (!isTauri()) {
    const dataStr =
      format === "json"
        ? JSON.stringify(results, null, 2)
        : "server_id,name,station,status,latency_ms\n" +
          results.map((r) => `${r.server_id},${r.name},${r.station},${r.status},${r.latency_ms ?? ""}`).join("\n");
    const blob = new Blob([dataStr], { type: format === "json" ? "application/json" : "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `nord_results.${format}`;
    a.click();
    URL.revokeObjectURL(url);
    return `nord_results.${format}`;
  }
  return await invoke<string>("export_results_file", {
    results,
    format,
    targetPath,
  });
}

export async function getAppVersion(): Promise<string> {
  if (!isTauri()) return "0.1.0-web";
  return await invoke<string>("get_app_version");
}

// -----------------------------------------------------------------------------
// Auto Updater API
// -----------------------------------------------------------------------------

export interface UpdateInfo {
  available: boolean;
  version?: string;
  body?: string;
  date?: string;
}

export async function checkForAppUpdates(): Promise<UpdateInfo> {
  if (!isTauri()) {
    return { available: false };
  }
  try {
    const update = await check();
    if (update?.available) {
      return {
        available: true,
        version: update.version,
        body: update.body ?? undefined,
        date: update.date ?? undefined,
      };
    }
    return { available: false };
  } catch (err) {
    console.warn("Failed checking for updates:", err);
    return { available: false };
  }
}

export async function installAppUpdate(): Promise<void> {
  if (!isTauri()) return;
  try {
    const update = await check();
    if (update?.available) {
      await update.downloadAndInstall();
    }
  } catch (err) {
    console.error("Failed to install update:", err);
    throw err;
  }
}

// -----------------------------------------------------------------------------
// Event Listeners
// -----------------------------------------------------------------------------

export async function onScanResult(callback: (result: TestResult) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return await listen<TestResult>("scan-result", (event) => {
    callback(event.payload);
  });
}

export async function onScanProgress(callback: (progress: ScanProgress) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return await listen<ScanProgress>("scan-progress", (event) => {
    callback(event.payload);
  });
}

export async function onScanFinished(callback: () => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  return await listen("scan-finished", () => {
    callback();
  });
}

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

const MOCK_SERVERS: ServerInfo[] = [
  {
    id: 1001,
    name: "Germany #1001",
    hostname: "de1001.nordvpn.com",
    station: "185.123.45.67",
    country_name: "Germany",
    country_code: "DE",
    city: "Frankfurt",
    load: 18,
    technologies: ["openvpn_xor", "nordwhisper"],
    groups: ["legacy_p2p", "legacy_obfuscated"],
  },
  {
    id: 8025,
    name: "United States #8025",
    hostname: "us8025.nordvpn.com",
    station: "89.187.177.241",
    country_name: "United States",
    country_code: "US",
    city: "New York",
    load: 24,
    technologies: ["nordwhisper", "openvpn_xor"],
    groups: ["legacy_standard", "legacy_p2p"],
  },
  {
    id: 2410,
    name: "United Kingdom #2410",
    hostname: "uk2410.nordvpn.com",
    station: "194.35.233.12",
    country_name: "United Kingdom",
    country_code: "GB",
    city: "London",
    load: 31,
    technologies: ["nordwhisper"],
    groups: ["legacy_p2p"],
  },
  {
    id: 1120,
    name: "Netherlands #1120",
    hostname: "nl1120.nordvpn.com",
    station: "185.246.188.75",
    country_name: "Netherlands",
    country_code: "NL",
    city: "Amsterdam",
    load: 15,
    technologies: ["nordwhisper", "openvpn_xor"],
    groups: ["legacy_standard"],
  },
  {
    id: 750,
    name: "France #750",
    hostname: "fr750.nordvpn.com",
    station: "185.220.101.5",
    country_name: "France",
    country_code: "FR",
    city: "Paris",
    load: 29,
    technologies: ["nordwhisper"],
    groups: ["legacy_p2p"],
  },
  {
    id: 320,
    name: "Switzerland #320",
    hostname: "ch320.nordvpn.com",
    station: "178.249.214.66",
    country_name: "Switzerland",
    country_code: "CH",
    city: "Zurich",
    load: 22,
    technologies: ["nordwhisper"],
    groups: ["legacy_standard"],
  },
  {
    id: 980,
    name: "Canada #980",
    hostname: "ca980.nordvpn.com",
    station: "198.54.135.18",
    country_name: "Canada",
    country_code: "CA",
    city: "Montreal",
    load: 38,
    technologies: ["openvpn_xor"],
    groups: ["legacy_p2p"],
  },
  {
    id: 610,
    name: "Sweden #610",
    hostname: "se610.nordvpn.com",
    station: "185.213.155.82",
    country_name: "Sweden",
    country_code: "SE",
    city: "Stockholm",
    load: 19,
    technologies: ["nordwhisper"],
    groups: ["legacy_standard"],
  },
  {
    id: 540,
    name: "Japan #540",
    hostname: "jp540.nordvpn.com",
    station: "103.152.220.91",
    country_name: "Japan",
    country_code: "JP",
    city: "Tokyo",
    load: 41,
    technologies: ["nordwhisper", "openvpn_xor"],
    groups: ["legacy_p2p"],
  },
  {
    id: 410,
    name: "Singapore #410",
    hostname: "sg410.nordvpn.com",
    station: "103.253.25.10",
    country_name: "Singapore",
    country_code: "SG",
    city: "Singapore",
    load: 35,
    technologies: ["nordwhisper"],
    groups: ["legacy_standard"],
  },
  {
    id: 390,
    name: "Australia #390",
    hostname: "au390.nordvpn.com",
    station: "103.107.198.15",
    country_name: "Australia",
    country_code: "AU",
    city: "Sydney",
    load: 48,
    technologies: ["nordwhisper"],
    groups: ["legacy_p2p"],
  },
  {
    id: 120,
    name: "Brazil #120",
    hostname: "br120.nordvpn.com",
    station: "185.216.140.22",
    country_name: "Brazil",
    country_code: "BR",
    city: "Sao Paulo",
    load: 55,
    technologies: ["openvpn_xor"],
    groups: ["legacy_standard"],
  },
];

const MOCK_RESULTS: TestResult[] = [
  {
    server_id: 1001,
    name: "Germany #1001",
    hostname: "de1001.nordvpn.com",
    station: "185.123.45.67",
    country_name: "Germany",
    country_code: "DE",
    city: "Frankfurt",
    load: 18,
    port: 443,
    status: "ONLINE",
    latency_ms: 28.4,
    tested_at: Math.floor(Date.now() / 1000) - 120,
    is_cached: false,
    features: ["Obfuscated", "NordWhisper"],
  },
  {
    server_id: 8025,
    name: "United States #8025",
    hostname: "us8025.nordvpn.com",
    station: "89.187.177.241",
    country_name: "United States",
    country_code: "US",
    city: "New York",
    load: 24,
    port: 443,
    status: "ONLINE",
    latency_ms: 31.2,
    tested_at: Math.floor(Date.now() / 1000) - 110,
    is_cached: false,
    features: ["NordWhisper", "Obfuscated"],
  },
  {
    server_id: 2410,
    name: "United Kingdom #2410",
    hostname: "uk2410.nordvpn.com",
    station: "194.35.233.12",
    country_name: "United Kingdom",
    country_code: "GB",
    city: "London",
    load: 31,
    port: 443,
    status: "ONLINE",
    latency_ms: 34.8,
    tested_at: Math.floor(Date.now() / 1000) - 100,
    is_cached: false,
    features: ["NordWhisper"],
  },
  {
    server_id: 1120,
    name: "Netherlands #1120",
    hostname: "nl1120.nordvpn.com",
    station: "185.246.188.75",
    country_name: "Netherlands",
    country_code: "NL",
    city: "Amsterdam",
    load: 15,
    port: 443,
    status: "ONLINE",
    latency_ms: 39.5,
    tested_at: Math.floor(Date.now() / 1000) - 90,
    is_cached: false,
    features: ["NordWhisper", "Obfuscated"],
  },
  {
    server_id: 750,
    name: "France #750",
    hostname: "fr750.nordvpn.com",
    station: "185.220.101.5",
    country_name: "France",
    country_code: "FR",
    city: "Paris",
    load: 29,
    port: 443,
    status: "ONLINE",
    latency_ms: 42.1,
    tested_at: Math.floor(Date.now() / 1000) - 80,
    is_cached: false,
    features: ["NordWhisper"],
  },
  {
    server_id: 320,
    name: "Switzerland #320",
    hostname: "ch320.nordvpn.com",
    station: "178.249.214.66",
    country_name: "Switzerland",
    country_code: "CH",
    city: "Zurich",
    load: 22,
    port: 443,
    status: "ONLINE",
    latency_ms: 46.7,
    tested_at: Math.floor(Date.now() / 1000) - 70,
    is_cached: false,
    features: ["NordWhisper"],
  },
  {
    server_id: 980,
    name: "Canada #980",
    hostname: "ca980.nordvpn.com",
    station: "198.54.135.18",
    country_name: "Canada",
    country_code: "CA",
    city: "Montreal",
    load: 38,
    port: 443,
    status: "ONLINE",
    latency_ms: 52.3,
    tested_at: Math.floor(Date.now() / 1000) - 60,
    is_cached: false,
    features: ["Obfuscated"],
  },
  {
    server_id: 610,
    name: "Sweden #610",
    hostname: "se610.nordvpn.com",
    station: "185.213.155.82",
    country_name: "Sweden",
    country_code: "SE",
    city: "Stockholm",
    load: 19,
    port: 443,
    status: "ONLINE",
    latency_ms: 58.9,
    tested_at: Math.floor(Date.now() / 1000) - 50,
    is_cached: false,
    features: ["NordWhisper"],
  },
  {
    server_id: 540,
    name: "Japan #540",
    hostname: "jp540.nordvpn.com",
    station: "103.152.220.91",
    country_name: "Japan",
    country_code: "JP",
    city: "Tokyo",
    load: 41,
    port: 443,
    status: "ONLINE",
    latency_ms: 72.4,
    tested_at: Math.floor(Date.now() / 1000) - 40,
    is_cached: false,
    features: ["NordWhisper", "Obfuscated"],
  },
  {
    server_id: 410,
    name: "Singapore #410",
    hostname: "sg410.nordvpn.com",
    station: "103.253.25.10",
    country_name: "Singapore",
    country_code: "SG",
    city: "Singapore",
    load: 35,
    port: 443,
    status: "ONLINE",
    latency_ms: 86.2,
    tested_at: Math.floor(Date.now() / 1000) - 30,
    is_cached: false,
    features: ["NordWhisper"],
  },
  {
    server_id: 390,
    name: "Australia #390",
    hostname: "au390.nordvpn.com",
    station: "103.107.198.15",
    country_name: "Australia",
    country_code: "AU",
    city: "Sydney",
    load: 48,
    port: 443,
    status: "ONLINE",
    latency_ms: 142.6,
    tested_at: Math.floor(Date.now() / 1000) - 20,
    is_cached: false,
    features: ["NordWhisper"],
  },
  {
    server_id: 120,
    name: "Brazil #120",
    hostname: "br120.nordvpn.com",
    station: "185.216.140.22",
    country_name: "Brazil",
    country_code: "BR",
    city: "Sao Paulo",
    load: 55,
    port: 443,
    status: "ONLINE",
    latency_ms: 178.3,
    tested_at: Math.floor(Date.now() / 1000) - 10,
    is_cached: false,
    features: ["Obfuscated"],
  },
];

export async function loadServers(): Promise<ServerInfo[]> {
  if (!isTauri()) {
    return MOCK_SERVERS;
  }
  return await invoke<ServerInfo[]>("load_servers");
}

export async function getCachedResults(ttlSecs?: number): Promise<TestResult[]> {
  if (!isTauri()) {
    return MOCK_RESULTS;
  }
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

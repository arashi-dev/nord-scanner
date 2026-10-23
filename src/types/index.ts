export type TestStatus =
  | "ONLINE"
  | "TIMEOUT"
  | "REFUSED"
  | "UNREACHABLE"
  | "ERROR"
  | "PENDING";

export interface ServerInfo {
  id: number;
  name: string;
  hostname: string;
  station: string;
  country_name: string;
  country_code: string;
  city: string;
  load: number;
  technologies: string[];
  groups: string[];
}

export interface TestResult {
  server_id: number;
  name: string;
  hostname: string;
  station: string;
  country_name: string;
  country_code: string;
  city: string;
  load: number;
  port: number;
  status: TestStatus;
  latency_ms: number | null;
  tested_at: number;
  is_cached: boolean;
  features: string[];
  error_msg?: string | null;
}

export interface MetadataInfo {
  exists: boolean;
  count: number;
  last_modified: number | null;
  age_seconds: number | null;
}

export interface RefreshStatus {
  total_servers: number;
  refreshed_at: number;
  age_seconds: number;
}

export interface ScanProgress {
  total: number;
  scanned: number;
  online: number;
  offline: number;
  avg_latency_ms: number;
  min_latency_ms: number;
  max_latency_ms: number;
  speed_eps: number;
  elapsed_secs: number;
  is_running: boolean;
  is_paused: boolean;
}

export interface FilterSettings {
  searchQuery: string;
  country: string; // 'ALL' or code like 'US', 'DE', etc.
  port: number; // default 443
  timeout: number; // default 30 seconds!
  concurrency: number; // default 200
  mode: "tcp" | "tls";
  serverType: string; // 'ALL' | 'obfuscated' | 'nordwhisper' | 'p2p' | 'double_vpn' | 'onion'
  statusFilter: "ALL" | "ONLINE" | "TIMEOUT" | "ERROR";
  maxLatency: number; // 0 means any
  sortBy: "latency_asc" | "latency_desc" | "load_asc" | "name_asc" | "country_asc";
}

export const DEFAULT_FILTERS: FilterSettings = {
  searchQuery: "",
  country: "ALL",
  port: 443,
  timeout: 30, // Default timeout must be 30 seconds
  concurrency: 200,
  mode: "tcp",
  serverType: "ALL",
  statusFilter: "ALL",
  maxLatency: 0,
  sortBy: "latency_asc",
};

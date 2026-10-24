# Spec Delta

## Purpose

The `server-catalog-mirror` capability ensures reliable and fast NordVPN server catalog discovery by pre-fetching, optimizing, and distributing catalog snapshots across resilient free CDNs, providing automatic client-side fallback when direct API access is restricted.

## ADDED Requirements

### Requirement: Scheduled Server Catalog Synchronization Pipeline
The CI/CD system SHALL periodically retrieve the complete NordVPN server catalog, sanitize and minify the payload to essential attributes, compress the resulting JSON, and publish artifacts to distribution channels.

#### Scenario: Scheduled Execution with Configurable Interval
- **WHEN** the scheduled workflow triggers
- **THEN** it checks the repository variable `SYNC_INTERVAL_HOURS` (defaulting to 6 hours) against the timestamp of the last successful sync, and if the interval has elapsed, it fetches `https://api.nordvpn.com/v1/servers?limit=0`, filters out unneeded fields, produces `servers.json` and `servers.json.gz`, updates the `servers-cache` GitHub Release asset, and force-pushes to the orphan branch `servers-data`.

#### Scenario: Manual Workflow Dispatch
- **WHEN** a maintainer triggers the workflow via `workflow_dispatch`
- **THEN** it executes the sync pipeline immediately regardless of the elapsed interval.

### Requirement: Multi-Tier Candidate Fallback Cascade
The client application SHALL attempt to fetch the server catalog through an ordered list of candidate endpoints with short per-endpoint timeouts before falling back to local persistent storage.

#### Scenario: Direct API Accessible
- **WHEN** `api.nordvpn.com` is reachable and returns HTTP 200 within 4 seconds
- **THEN** the client parses the catalog, updates the local cache file, and completes the refresh.

#### Scenario: Direct API Blocked or Timed Out
- **WHEN** `api.nordvpn.com` times out after 4 seconds or encounters a connection error
- **THEN** the client immediately attempts candidate #2 (GitHub Releases), followed by candidate #3 (jsDelivr CDN), candidate #4 (Statically CDN), and candidate #5 (GitHub Raw), each with a 5-second timeout, until a valid response is received.

#### Scenario: All Remote Mirrors Inaccessible
- **WHEN** all remote candidates fail or the device is offline
- **THEN** the client loads the existing local cache from disk without crashing, informing the user that offline cached data is in use.

### Requirement: Transparent In-Memory Gzip Decompression
The client application SHALL support fetching compressed `.json.gz` payloads and decompressing them in-memory.

#### Scenario: Receiving Gzipped Payload
- **WHEN** the client downloads a compressed catalog payload with gzip header magic bytes or `.gz` URL
- **THEN** it transparently decompresses the byte stream in-memory and deserializes into `Vec<ServerInfo>` without creating temporary files on disk.

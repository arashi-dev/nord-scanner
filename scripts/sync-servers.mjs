#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const NORD_API_URL = 'https://api.nordvpn.com/v1/servers?limit=0';
const OUT_DIR = path.resolve(process.cwd(), 'dist-data');

async function syncServers() {
  console.log(`[1/4] Fetching complete server catalog from ${NORD_API_URL}...`);
  const response = await fetch(NORD_API_URL, {
    headers: {
      'User-Agent': 'NordScanner-Sync/1.0',
      'Accept': 'application/json',
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to fetch from NordVPN API: ${response.status} ${response.statusText}`);
  }

  const rawList = await response.json();
  if (!Array.isArray(rawList) || rawList.length === 0) {
    throw new Error('Received empty or invalid server catalog payload from NordVPN API');
  }
  console.log(`[2/4] Received ${rawList.length} raw server records.`);

  const minified = rawList
    .map((s) => {
      const loc = s.locations && s.locations.length > 0 ? s.locations[0] : null;
      return {
        id: s.id,
        name: s.name || `Server #${s.id}`,
        hostname: s.hostname || '',
        station: s.station || '',
        country_name: loc?.country?.name || 'Unknown',
        country_code: (loc?.country?.code || '??').toUpperCase(),
        city: loc?.city?.name || '',
        load: s.load || 0,
        technologies: (s.technologies || []).map((t) => t.identifier).filter(Boolean),
        groups: (s.groups || []).map((g) => g.identifier).filter(Boolean),
      };
    })
    .filter((s) => s.station && s.station.trim().length > 0);

  console.log(`[3/4] Parsed ${minified.length} valid server records.`);
  if (minified.length < 1000) {
    throw new Error(`Aborting: Parsed server count (${minified.length}) is below safety threshold (1000).`);
  }

  fs.mkdirSync(OUT_DIR, { recursive: true });

  const jsonStr = JSON.stringify(minified);
  const outJsonPath = path.join(OUT_DIR, 'servers.json');
  fs.writeFileSync(outJsonPath, jsonStr, 'utf8');

  const gzipped = zlib.gzipSync(Buffer.from(jsonStr), { level: 9 });
  const outGzPath = path.join(OUT_DIR, 'servers.json.gz');
  fs.writeFileSync(outGzPath, gzipped);

  const mbSize = (Buffer.byteLength(jsonStr) / (1024 * 1024)).toFixed(2);
  const kbGzSize = (gzipped.length / 1024).toFixed(2);
  console.log(`[4/4] Successfully generated:\n  - ${outJsonPath} (${mbSize} MB)\n  - ${outGzPath} (${kbGzSize} KB)`);
}

syncServers().catch((err) => {
  console.error('[Error] Server sync failed:', err);
  process.exit(1);
});

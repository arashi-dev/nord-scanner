"""NordVPN API client for retrieving server lists with local caching."""

import json
import os
import time
from pathlib import Path
from typing import List, Optional, Callable
import httpx

from .models import ServerInfo

NORD_SERVERS_URL = "https://api.nordvpn.com/v1/servers?limit=0"
DEFAULT_CACHE_DIR = Path(".cache")
SERVER_META_CACHE_FILE = DEFAULT_CACHE_DIR / "nord_servers_metadata.json"
META_CACHE_TTL_HOURS = 24.0


def parse_server(s: dict) -> Optional[ServerInfo]:
    """Parse raw JSON dict from NordVPN API into ServerInfo."""
    station = s.get("station")
    if not station:
        return None

    locations = s.get("locations") or []
    loc = locations[0] if locations else {}
    country_obj = loc.get("country") or {}
    city_obj = loc.get("city") or {}

    country_name = country_obj.get("name") or "Unknown"
    country_code = country_obj.get("code") or "??"
    city_name = city_obj.get("name") or ""

    tech_identifiers = [
        t.get("identifier", "")
        for t in s.get("technologies") or []
        if isinstance(t, dict) and t.get("identifier")
    ]

    return ServerInfo(
        id=int(s["id"]),
        name=s.get("name") or f"Server #{s['id']}",
        hostname=s.get("hostname") or "",
        station=str(station),
        country_name=country_name,
        country_code=country_code.upper(),
        city=city_name,
        load=int(s.get("load") or 0),
        technologies=tech_identifiers,
    )


def fetch_all_servers(
    force_refresh: bool = False,
    progress_callback: Optional[Callable[[str], None]] = None,
    country_filter: Optional[List[str]] = None,
) -> List[ServerInfo]:
    """
    Fetch all NordVPN servers.
    Uses local cached metadata if fresh (<24h) unless force_refresh is True.
    """
    DEFAULT_CACHE_DIR.mkdir(parents=True, exist_ok=True)
    raw_data = None

    if not force_refresh and SERVER_META_CACHE_FILE.exists():
        try:
            mtime = SERVER_META_CACHE_FILE.stat().st_mtime
            age_hours = (time.time() - mtime) / 3600.0
            if age_hours < META_CACHE_TTL_HOURS:
                if progress_callback:
                    progress_callback(f"Loading server metadata from local cache ({age_hours:.1f}h old)...")
                with open(SERVER_META_CACHE_FILE, "r", encoding="utf-8") as f:
                    raw_data = json.load(f)
        except Exception as e:
            if progress_callback:
                progress_callback(f"Failed to read local server cache: {e}. Refetching...")
            raw_data = None

    if raw_data is None:
        if progress_callback:
            progress_callback("Connecting to NordVPN API to fetch complete server catalog...")
        try:
            with httpx.Client(timeout=30.0) as client:
                resp = client.get(NORD_SERVERS_URL)
                resp.raise_for_status()
                raw_data = resp.json()

            # Cache the server metadata locally
            try:
                with open(SERVER_META_CACHE_FILE, "w", encoding="utf-8") as f:
                    json.dump(raw_data, f)
            except Exception:
                pass
        except Exception as e:
            if SERVER_META_CACHE_FILE.exists():
                if progress_callback:
                    progress_callback(f"API fetch failed ({e}). Falling back to cached servers...")
                with open(SERVER_META_CACHE_FILE, "r", encoding="utf-8") as f:
                    raw_data = json.load(f)
            else:
                raise RuntimeError(f"Failed to fetch NordVPN servers and no cache available: {e}")

    servers: List[ServerInfo] = []
    filter_set = {c.strip().upper() for c in country_filter} if country_filter else None

    for item in raw_data:
        parsed = parse_server(item)
        if parsed is None:
            continue
        if filter_set and parsed.country_code not in filter_set:
            continue
        servers.append(parsed)

    if progress_callback:
        progress_callback(f"Ready: Loaded {len(servers)} servers.")

    return servers

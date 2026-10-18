"""Main application coordinator orchestrating API, Scanner, Cache, and TUI."""

import asyncio
import os
import signal
import sys
import time
from pathlib import Path
from typing import List, Optional

from rich.console import Console
from rich.live import Live

from .api import fetch_all_servers
from .cache import CacheManager
from .exporter import export_results
from .keyboard import KeyboardListener
from .models import ServerInfo, TestResult
from .scanner import BulkScanner
from .tui import AppState, render_ui


def parse_ttl(ttl_input: str) -> float:
    """Parse human readable TTL strings like '30m', '6h', '1d', or seconds."""
    s = str(ttl_input).strip().lower()
    if s.endswith("s"):
        return max(1.0, float(s[:-1]))
    elif s.endswith("m"):
        return max(1.0, float(s[:-1]) * 60.0)
    elif s.endswith("h"):
        return max(1.0, float(s[:-1]) * 3600.0)
    elif s.endswith("d"):
        return max(1.0, float(s[:-1]) * 86400.0)
    return max(1.0, float(s))


class AppRunner:
    """Manages the full lifecycle of the TUI application."""

    def __init__(
        self,
        concurrency: int = 500,
        port: int = 443,
        timeout: float = 3.0,
        ttl_str: str = "6h",
        force_retest: bool = False,
        refresh_servers: bool = False,
        mode: str = "tcp",
        country_filter: Optional[List[str]] = None,
        feature_filter: Optional[List[str]] = None,
        csv_export: Optional[str] = "nord_results.csv",
        json_export: Optional[str] = "nord_results.json",
        auto_exit: bool = False,
    ):
        self.concurrency = concurrency
        self.port = port
        self.timeout = timeout
        self.ttl_str = ttl_str
        self.ttl_seconds = parse_ttl(ttl_str)
        self.force_retest = force_retest
        self.refresh_servers = refresh_servers
        self.mode = mode
        self.country_filter = country_filter
        self.feature_filter = feature_filter
        self.csv_export = csv_export
        self.json_export = json_export
        self.auto_exit = auto_exit

        self.console = Console()
        self.cache_manager = CacheManager()
        self.app_state: Optional[AppState] = None
        self.scanner: Optional[BulkScanner] = None
        self.keyboard_listener: Optional[KeyboardListener] = None
        self.all_servers: List[ServerInfo] = []

        self._shutdown_event = asyncio.Event()
        self._retest_requested = asyncio.Event()

    def handle_keypress(self, key: str):
        """Handle hotkeys from the keyboard listener."""
        key = key.lower()
        if key in ("q", "\x03"):  # 'q' or Ctrl+C
            if self.app_state:
                self.app_state.status_message = "Exiting and saving results..."
            self._shutdown_event.set()
            if self.scanner:
                self.scanner.stop()
        elif key == "p":
            if self.scanner and self.app_state:
                is_paused = self.scanner.toggle_pause()
                self.app_state.is_paused = is_paused
                self.app_state.status_message = "PAUSED (press [P] to resume)" if is_paused else "Resumed testing..."
        elif key == "r":
            if self.app_state:
                self.app_state.status_message = "Forcing full re-test: clearing cache..."
            self._retest_requested.set()
            if self.scanner:
                self.scanner.stop()
        elif key == "e":
            self._perform_export(interactive=True)

    def _perform_export(self, interactive: bool = False):
        if not self.app_state:
            return
        results = list(self.app_state.results_map.values())
        if not results:
            if interactive:
                self.app_state.status_message = "No results to export yet."
            return

        csv_p = Path(self.csv_export) if self.csv_export else None
        json_p = Path(self.json_export) if self.json_export else None
        paths = export_results(results, csv_path=csv_p, json_path=json_p)
        msg_parts = [Path(p).name for p in paths.values()]
        if msg_parts:
            self.app_state.status_message = f"Saved: {', '.join(msg_parts)}"

    async def _render_loop(self, live: Live):
        """Continuously re-render TUI at ~10 FPS."""
        while not self._shutdown_event.is_set():
            if self.app_state:
                width = self.console.size.width
                live.update(render_ui(self.app_state, console_width=width))
            await asyncio.sleep(0.1)

    async def _run_scanner_session(self, servers_to_test: List[ServerInfo]):
        """Run a scanner session on the given list of servers."""
        if not servers_to_test:
            if self.app_state:
                self.app_state.is_completed = True
                self.app_state.status_message = "All servers fresh in cache! Press [R] to force retest."
            return

        def on_result_cb(res: TestResult):
            if self.app_state:
                self.app_state.record_result(res)

        actual_concurrency = min(self.concurrency, len(servers_to_test)) if self.concurrency > 0 else len(servers_to_test)
        self.scanner = BulkScanner(
            servers=servers_to_test,
            cache_manager=self.cache_manager,
            concurrency=actual_concurrency,
            port=self.port,
            timeout=self.timeout,
            mode=self.mode,
            on_result=on_result_cb,
        )

        if self.app_state:
            self.app_state.status_message = f"Scanning {len(servers_to_test):,} servers ({actual_concurrency} workers)..."

        await self.scanner.run()

        if self.app_state and not self._shutdown_event.is_set():
            self.app_state.is_completed = True
            self.app_state.status_message = "Scan complete! Results cached."

    async def run(self):
        """Main asynchronous runner."""
        # Setup keyboard listener
        self.keyboard_listener = KeyboardListener(self.handle_keypress)
        await self.keyboard_listener.start()

        # Initial message
        filter_desc = ",".join(self.country_filter).upper() if self.country_filter else "ALL"
        type_desc = "+".join(f.title() for f in self.feature_filter) if self.feature_filter else "ALL"
        self.console.print(f"[bold cyan]Initializing NordVPN Bulk Tester...[/bold cyan]")
        self.console.print(
            f"[dim]Country filter: {filter_desc} | Type filter: {type_desc} | "
            f"Mode: {self.mode.upper()} | Concurrency: {self.concurrency}[/dim]"
        )

        # If user passed force_retest CLI flag, clear the cache upfront
        if self.force_retest:
            self.cache_manager.clear_cache()
            self.console.print("[yellow]Cache cleared (--force specified). Retesting all servers.[/yellow]")

        # Fetch server list
        self.all_servers = fetch_all_servers(
            force_refresh=self.refresh_servers,
            progress_callback=lambda msg: self.console.print(f"[dim]{msg}[/dim]"),
            country_filter=self.country_filter,
            feature_filter=self.feature_filter,
        )

        if not self.all_servers:
            self.console.print("[bold red]No servers matched the criteria or failed to load server list.[/bold red]")
            await self.keyboard_listener.stop()
            return

        # Prepare initial cache state
        fresh_cache = self.cache_manager.get_fresh_cache(self.ttl_seconds)

        # Categorize into cached vs untested for current scope
        relevant_cache = {s.id: fresh_cache[s.id] for s in self.all_servers if s.id in fresh_cache}
        cached_servers = [s for s in self.all_servers if s.id in relevant_cache]
        untested_servers = [s for s in self.all_servers if s.id not in relevant_cache]

        self.app_state = AppState(
            total_servers=len(self.all_servers),
            cached_count=len(cached_servers),
            pending_count=len(untested_servers),
            concurrency=self.concurrency,
            port=self.port,
            ttl_display=self.ttl_str,
            mode=self.mode,
            country_filter_str=filter_desc,
            feature_filter_str=type_desc,
        )
        self.app_state.register_cached_results(relevant_cache)

        # Start TUI Live display
        with Live(render_ui(self.app_state, self.console.size.width), console=self.console, refresh_per_second=10) as live:
            render_task = asyncio.create_task(self._render_loop(live))

            try:
                while not self._shutdown_event.is_set():
                    self._retest_requested.clear()
                    # Run scan on pending servers
                    scan_task = asyncio.create_task(self._run_scanner_session(untested_servers))

                    # Wait for scan to complete OR shutdown OR retest request
                    while not scan_task.done():
                        if self._shutdown_event.is_set():
                            scan_task.cancel()
                            break
                        if self._retest_requested.is_set():
                            scan_task.cancel()
                            break
                        await asyncio.sleep(0.1)

                    if self._shutdown_event.is_set():
                        break

                    if self._retest_requested.is_set():
                        # Wipe cache and re-queue all servers
                        self.cache_manager.clear_cache()
                        self.app_state = AppState(
                            total_servers=len(self.all_servers),
                            cached_count=0,
                            pending_count=len(self.all_servers),
                            concurrency=self.concurrency,
                            port=self.port,
                            ttl_display=self.ttl_str,
                            mode=self.mode,
                            country_filter_str=filter_desc,
                            feature_filter_str=type_desc,
                        )
                        untested_servers = list(self.all_servers)
                        continue

                    # If auto-exit requested on complete
                    if self.auto_exit:
                        break

                    # Keep UI open until user exits
                    while not self._shutdown_event.is_set() and not self._retest_requested.is_set():
                        await asyncio.sleep(0.1)

            finally:
                render_task.cancel()
                try:
                    await render_task
                except asyncio.CancelledError:
                    pass

        # Cleanup keyboard listener
        await self.keyboard_listener.stop()

        # Final export
        self._perform_export()
        self._print_exit_summary()

    def _print_exit_summary(self):
        """Print neat terminal summary on exit."""
        if not self.app_state:
            return
        self.console.print("\n[bold cyan]=== NordVPN Test Run Complete ===[/bold cyan]")
        self.console.print(f"Total Servers:   {self.app_state.total_servers:,}")
        self.console.print(f"Passed (Cached): {self.app_state.cached_count:,}")
        self.console.print(f"Online:          [green]{self.app_state.online_count:,}[/green]")
        self.console.print(f"Offline/Timeout: [red]{self.app_state.offline_count:,}[/red]")
        if self.app_state.min_latency is not None:
            self.console.print(
                f"Latency:         Min: [green]{self.app_state.min_latency:.1f}ms[/green] | "
                f"Avg: [cyan]{self.app_state.avg_latency:.1f}ms[/cyan] | "
                f"Max: [magenta]{self.app_state.max_latency:.1f}ms[/magenta]"
            )
        if self.csv_export and Path(self.csv_export).exists():
            self.console.print(f"CSV Export:      [dim]{Path(self.csv_export).resolve()}[/dim]")
        if self.json_export and Path(self.json_export).exists():
            self.console.print(f"JSON Export:     [dim]{Path(self.json_export).resolve()}[/dim]")

"""Rich Terminal User Interface for NordVPN bulk connection tester."""

import collections
import statistics
import time
from typing import Deque, Dict, List, Optional

from rich.console import Console, RenderableType, Group
from rich.panel import Panel
from rich.table import Table
from rich.text import Text
from rich.layout import Layout
from rich.columns import Columns

from .models import ServerInfo, TestResult, TestStatus


class AppState:
    """Encapsulates all reactive state for the TUI."""

    def __init__(
        self,
        total_servers: int,
        cached_count: int,
        pending_count: int,
        concurrency: int,
        port: int,
        ttl_display: str,
        mode: str,
        country_filter_str: str = "ALL",
    ):
        self.total_servers = total_servers
        self.cached_count = cached_count
        self.pending_count = pending_count
        self.concurrency = concurrency
        self.port = port
        self.ttl_display = ttl_display
        self.mode = mode.upper()
        self.country_filter_str = country_filter_str

        self.tested_count = cached_count
        self.online_count = 0
        self.offline_count = 0

        self.start_time = time.time()
        self.completed_time: Optional[float] = None
        self.is_paused = False
        self.is_completed = False
        self.status_message = "Initializing bulk scanner..."

        # Live results tracking
        self.results_map: Dict[int, TestResult] = {}
        self.recent_stream: Deque[TestResult] = collections.deque(maxlen=10)
        self.top_servers: List[TestResult] = []

    def register_cached_results(self, cached: Dict[int, TestResult]):
        """Register pre-existing fresh results from SQLite cache."""
        for r in cached.values():
            self.results_map[r.server_id] = r
            if r.status == TestStatus.ONLINE:
                self.online_count += 1
            else:
                self.offline_count += 1
        self._update_top_servers()

    def record_result(self, result: TestResult):
        """Record a live test result."""
        # If server was already in map as cached, replace it
        prev = self.results_map.get(result.server_id)
        if prev is None:
            self.tested_count += 1
        else:
            if prev.status == TestStatus.ONLINE:
                self.online_count -= 1
            else:
                self.offline_count -= 1

        self.results_map[result.server_id] = result
        if result.status == TestStatus.ONLINE:
            self.online_count += 1
        else:
            self.offline_count += 1

        self.recent_stream.appendleft(result)
        self._update_top_servers()

        if self.tested_count >= self.total_servers and not self.is_completed:
            self.is_completed = True
            self.completed_time = time.time()
            self.status_message = "All servers tested! Test completed successfully."

    def _update_top_servers(self):
        online = [
            r for r in self.results_map.values()
            if r.status == TestStatus.ONLINE and r.latency_ms is not None
        ]
        online.sort(key=lambda r: r.latency_ms or 999999)
        self.top_servers = online[:10]

    @property
    def latencies(self) -> List[float]:
        return [
            r.latency_ms
            for r in self.results_map.values()
            if r.status == TestStatus.ONLINE and r.latency_ms is not None
        ]

    @property
    def min_latency(self) -> Optional[float]:
        lats = self.latencies
        return min(lats) if lats else None

    @property
    def avg_latency(self) -> Optional[float]:
        lats = self.latencies
        return statistics.mean(lats) if lats else None

    @property
    def max_latency(self) -> Optional[float]:
        lats = self.latencies
        return max(lats) if lats else None

    @property
    def elapsed_seconds(self) -> float:
        end = self.completed_time or time.time()
        return max(0.1, end - self.start_time)

    @property
    def tests_per_sec(self) -> float:
        # Rate of live tested servers (excluding instant cached)
        live_tested = max(0, self.tested_count - self.cached_count)
        return live_tested / self.elapsed_seconds

    @property
    def eta_seconds(self) -> Optional[float]:
        if self.is_completed or self.tested_count >= self.total_servers:
            return 0.0
        rate = self.tests_per_sec
        remaining = max(0, self.total_servers - self.tested_count)
        if rate > 0:
            return remaining / rate
        return None


def format_duration(seconds: Optional[float]) -> str:
    if seconds is None:
        return "--:--"
    s = int(seconds)
    m = s // 60
    return f"{m:02d}:{s % 60:02d}"


def get_latency_styled(latency_ms: Optional[float]) -> Text:
    """Format latency with color grading."""
    if latency_ms is None:
        return Text("---", style="dim")
    if latency_ms < 75.0:
        return Text(f"{latency_ms:5.1f} ms", style="bold green")
    elif latency_ms < 150.0:
        return Text(f"{latency_ms:5.1f} ms", style="cyan")
    elif latency_ms < 250.0:
        return Text(f"{latency_ms:5.1f} ms", style="yellow")
    else:
        return Text(f"{latency_ms:5.1f} ms", style="bold magenta")


def get_status_styled(status: TestStatus, is_cached: bool) -> Text:
    """Format status indicator with universal terminal safe characters."""
    if is_cached:
        return Text("[CACHED]", style="cyan")
    if status == TestStatus.ONLINE:
        return Text("[ONLINE]", style="bold green")
    elif status == TestStatus.TIMEOUT:
        return Text("[TIMEOUT]", style="bold yellow")
    elif status == TestStatus.REFUSED:
        return Text("[REFUSED]", style="bold red")
    elif status == TestStatus.UNREACHABLE:
        return Text("[UNREACH]", style="bold red")
    else:
        return Text(f"[{status.value[:7]}]", style="red")


def build_progress_bar(current: int, total: int, width: int = 30) -> str:
    """Build universal text progress bar."""
    if total <= 0:
        return "[" + " " * width + "]"
    fraction = min(1.0, max(0.0, current / total))
    filled_len = int(width * fraction)
    bar = "=" * filled_len + ("=" if filled_len == width else ">") + "-" * max(0, width - filled_len - 1)
    bar = bar[:width]
    pct = fraction * 100.0
    return f"[{bar}] {pct:5.1f}%"


def render_ui(state: AppState, console_width: int = 120) -> RenderableType:
    """Generate the complete Rich renderable for current app state."""
    # 1. Header Panel
    header_table = Table.grid(expand=True)
    header_table.add_column(justify="left", ratio=1)
    header_table.add_column(justify="right", ratio=1)

    title_text = Text.assemble(
        ("NORDVPN BULK CONNECTION BENCHMARK ", "bold cyan"),
        (f"[{state.mode}]", "bold yellow"),
    )
    right_meta = Text.assemble(
        ("Servers: ", "dim"),
        (f"{state.total_servers:,} ", "bold white"),
        ("Cached: ", "dim"),
        (f"{state.cached_count:,} ", "bold cyan"),
        ("Pending: ", "dim"),
        (f"{state.pending_count:,} ", "bold yellow"),
        ("Concurrency: ", "dim"),
        (f"{state.concurrency:,} ", "bold magenta"),
        ("Port: ", "dim"),
        (f"{state.port} ", "white"),
        ("TTL: ", "dim"),
        (f"{state.ttl_display}", "green"),
    )
    header_table.add_row(title_text, right_meta)
    header_panel = Panel(header_table, style="cyan", padding=(0, 1))

    # 2. Metrics & Progress Dashboard
    stats_table = Table.grid(expand=True, padding=(0, 1))
    for _ in range(4):
        stats_table.add_column(ratio=1)

    # Col 1: Progress
    pct = (state.tested_count / state.total_servers * 100.0) if state.total_servers else 0.0
    col1 = Text.assemble(
        ("Tested:   ", "dim"),
        (f"{state.tested_count:,} / {state.total_servers:,} ", "bold white"),
        (f"({pct:.1f}%)\n", "cyan"),
        ("Progress: ", "dim"),
        (build_progress_bar(state.tested_count, state.total_servers, width=22), "bold green"),
    )

    # Col 2: Online / Offline
    online_pct = (state.online_count / state.tested_count * 100.0) if state.tested_count else 0.0
    offline_pct = (state.offline_count / state.tested_count * 100.0) if state.tested_count else 0.0
    col2 = Text.assemble(
        ("Online:  ", "dim"),
        (f"{state.online_count:,} ", "bold green"),
        (f"({online_pct:.1f}%)\n", "green"),
        ("Offline: ", "dim"),
        (f"{state.offline_count:,} ", "bold red" if state.offline_count else "white"),
        (f"({offline_pct:.1f}%)", "red" if state.offline_count else "dim"),
    )

    # Col 3: Latencies
    min_lat = f"{state.min_latency:.1f} ms" if state.min_latency is not None else "---"
    avg_lat = f"{state.avg_latency:.1f} ms" if state.avg_latency is not None else "---"
    max_lat = f"{state.max_latency:.1f} ms" if state.max_latency is not None else "---"
    col3 = Text.assemble(
        ("Latency Min: ", "dim"),
        (f"{min_lat}\n", "bold green"),
        ("Avg / Max:   ", "dim"),
        (f"{avg_lat} ", "cyan"),
        ("/ ", "dim"),
        (f"{max_lat}", "magenta"),
    )

    # Col 4: Speed & Time
    speed_str = f"{state.tests_per_sec:.1f} /s"
    elapsed_str = format_duration(state.elapsed_seconds)
    eta_str = format_duration(state.eta_seconds)
    col4 = Text.assemble(
        ("Speed:   ", "dim"),
        (f"{speed_str}\n", "bold yellow"),
        ("Elapsed: ", "dim"),
        (f"{elapsed_str} ", "white"),
        ("ETA: ", "dim"),
        (f"{eta_str}", "yellow"),
    )

    stats_table.add_row(col1, col2, col3, col4)
    dashboard_panel = Panel(stats_table, title="[bold]Session Statistics[/bold]", style="blue", padding=(0, 1))

    # 3. Main Content: Recent Live Probes (Left) & Top 10 Leaderboard (Right)
    # Stream Table
    stream_table = Table(title="[bold]Live Probe Stream (Latest Tests)[/bold]", expand=True, box=None)
    stream_table.add_column("Country", ratio=2, overflow="ellipsis")
    stream_table.add_column("Server", ratio=3, overflow="ellipsis")
    stream_table.add_column("IP Address", ratio=2)
    stream_table.add_column("Status", ratio=2)
    stream_table.add_column("Latency", justify="right", ratio=2)
    stream_table.add_column("Src", justify="center", ratio=1)

    if not state.recent_stream:
        stream_table.add_row(
            Text("Waiting for probes...", style="dim"),
            Text("-", style="dim"),
            Text("-", style="dim"),
            Text("-", style="dim"),
            Text("-", style="dim"),
            Text("-", style="dim"),
        )
    else:
        for r in list(state.recent_stream)[:8]:
            country_fmt = f"[{r.country_code}] {r.country_name[:12]}"
            stream_table.add_row(
                Text(country_fmt, style="white"),
                Text(r.name, style="bold white", overflow="ellipsis"),
                Text(r.station, style="dim"),
                get_status_styled(r.status, r.is_cached),
                get_latency_styled(r.latency_ms),
                Text("Cache" if r.is_cached else "Live", style="cyan" if r.is_cached else "green"),
            )

    stream_panel = Panel(stream_table, style="white", padding=(0, 1))

    # Leaderboard Table
    top_table = Table(title="[bold]Top 10 Fastest Servers[/bold]", expand=True, box=None)
    top_table.add_column("#", justify="right", width=2)
    top_table.add_column("Country", ratio=2, overflow="ellipsis")
    top_table.add_column("Server", ratio=3, overflow="ellipsis")
    top_table.add_column("Latency", justify="right", ratio=2)
    top_table.add_column("Load", justify="right", ratio=1)

    if not state.top_servers:
        top_table.add_row(
            Text("-", style="dim"),
            Text("No results yet", style="dim"),
            Text("-", style="dim"),
            Text("-", style="dim"),
            Text("-", style="dim"),
        )
    else:
        for i, r in enumerate(state.top_servers[:8], start=1):
            rank_style = "bold yellow" if i == 1 else ("bold cyan" if i <= 3 else "white")
            load_style = "green" if r.load < 40 else ("yellow" if r.load < 75 else "red")
            top_table.add_row(
                Text(f"#{i}", style=rank_style),
                Text(f"[{r.country_code}] {r.country_name[:10]}", style="white"),
                Text(r.name, style="bold white", overflow="ellipsis"),
                get_latency_styled(r.latency_ms),
                Text(f"{r.load}%", style=load_style),
            )

    leaderboard_panel = Panel(top_table, style="yellow", padding=(0, 1))

    # Dual pane or stacked based on width
    if console_width >= 100:
        main_grid = Table.grid(expand=True)
        main_grid.add_column(ratio=6)
        main_grid.add_column(ratio=4)
        main_grid.add_row(stream_panel, leaderboard_panel)
        main_content = main_grid
    else:
        main_content = Group(stream_panel, leaderboard_panel)

    # 4. Footer & Control Hints
    footer_table = Table.grid(expand=True)
    footer_table.add_column(justify="left", ratio=1)
    footer_table.add_column(justify="right", ratio=1)

    # Status tag
    if state.is_paused:
        status_tag = Text.assemble(("[PAUSED] ", "bold yellow"), (state.status_message, "yellow"))
    elif state.is_completed:
        status_tag = Text.assemble(("[COMPLETE] ", "bold green"), (state.status_message, "green"))
    else:
        status_tag = Text.assemble(("[TESTING] ", "bold cyan"), (state.status_message, "white"))

    controls_text = Text.assemble(
        ("[Q]", "bold red"), (" Quit & Save  ", "dim"),
        ("[P]", "bold yellow"), (" Pause/Resume  ", "dim"),
        ("[R]", "bold cyan"), (" Retest All  ", "dim"),
        ("[E]", "bold green"), (" Export CSV/JSON", "dim"),
    )
    footer_table.add_row(status_tag, controls_text)
    footer_panel = Panel(footer_table, style="dim", padding=(0, 1))

    return Group(
        header_panel,
        dashboard_panel,
        main_content,
        footer_panel,
    )

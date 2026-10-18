"""Data models for NordVPN bulk tester."""

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Dict
import time


class TestStatus(str, Enum):
    ONLINE = "ONLINE"
    TIMEOUT = "TIMEOUT"
    REFUSED = "REFUSED"
    UNREACHABLE = "UNREACHABLE"
    ERROR = "ERROR"
    PENDING = "PENDING"


@dataclass
class ServerInfo:
    id: int
    name: str
    hostname: str
    station: str
    country_name: str
    country_code: str
    city: str
    load: int
    technologies: List[str] = field(default_factory=list)
    groups: List[str] = field(default_factory=list)

    @property
    def is_obfuscated(self) -> bool:
        """Check if server supports Obfuscated / XOR protocol."""
        return any("xor" in t.lower() or "obfusc" in t.lower() for t in self.technologies) or \
               any("obfusc" in g.lower() for g in self.groups)

    @property
    def is_nordwhisper(self) -> bool:
        """Check if server supports NordWhisper protocol."""
        return any("whisper" in t.lower() for t in self.technologies)

    @property
    def feature_tags(self) -> List[str]:
        """Return list of human-friendly feature tags."""
        tags = []
        if self.is_obfuscated:
            tags.append("Obfuscated")
        if self.is_nordwhisper:
            tags.append("NordWhisper")
        if any("p2p" in g.lower() for g in self.groups):
            tags.append("P2P")
        if any("double" in g.lower() for g in self.groups):
            tags.append("DoubleVPN")
        if any("onion" in g.lower() for g in self.groups):
            tags.append("Onion")
        return tags or ["Standard"]


@dataclass
class TestResult:
    server_id: int
    name: str
    hostname: str
    station: str
    country_name: str
    country_code: str
    city: str
    load: int
    port: int
    status: TestStatus
    latency_ms: Optional[float]  # round trip latency in milliseconds
    tested_at: float             # epoch timestamp in seconds
    is_cached: bool = False      # true if loaded from persistent cache
    features: List[str] = field(default_factory=list)
    error_msg: Optional[str] = None

    @property
    def age_seconds(self) -> float:
        return max(0.0, time.time() - self.tested_at)

    @property
    def age_formatted(self) -> str:
        s = int(self.age_seconds)
        if s < 60:
            return f"{s}s ago"
        m = s // 60
        if m < 60:
            return f"{m}m ago"
        h = m // 60
        return f"{h}h {m % 60}m ago"

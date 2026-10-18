"""Asynchronous bulk socket connection scanner with rate limiting and batch persistence."""

import asyncio
import ssl
import time
from typing import Callable, List, Optional, Tuple

from .cache import CacheManager
from .models import ServerInfo, TestResult, TestStatus


class BulkScanner:
    """Orchestrates concurrent connection tests across thousands of servers."""

    def __init__(
        self,
        servers: List[ServerInfo],
        cache_manager: CacheManager,
        concurrency: int = 500,
        port: int = 443,
        timeout: float = 3.0,
        mode: str = "tcp",
        on_result: Optional[Callable[[TestResult], None]] = None,
    ):
        self.servers = servers
        self.cache_manager = cache_manager
        self.concurrency = max(1, concurrency)
        self.port = port
        self.timeout = timeout
        self.mode = mode.lower()
        self.on_result = on_result

        self._semaphore = asyncio.Semaphore(self.concurrency)
        self._pause_event = asyncio.Event()
        self._pause_event.set()  # Unpaused initially
        self._stop_event = asyncio.Event()

        self._db_queue: asyncio.Queue[Optional[TestResult]] = asyncio.Queue()
        self._db_task: Optional[asyncio.Task] = None
        self._scanner_task: Optional[asyncio.Task] = None

        # Pre-build SSL context if in TLS mode
        self._ssl_ctx: Optional[ssl.SSLContext] = None
        if self.mode == "tls":
            self._ssl_ctx = ssl.create_default_context()
            self._ssl_ctx.check_hostname = False
            self._ssl_ctx.verify_mode = ssl.CERT_NONE

    @property
    def is_paused(self) -> bool:
        return not self._pause_event.is_set()

    def toggle_pause(self) -> bool:
        """Toggle pause state. Returns True if now paused, False if resumed."""
        if self._pause_event.is_set():
            self._pause_event.clear()
            return True
        else:
            self._pause_event.set()
            return False

    def stop(self):
        """Signal the scanner to stop immediately."""
        self._stop_event.set()
        self._pause_event.set()  # Unpause to let workers exit

    async def _probe_single(self, server: ServerInfo) -> TestResult:
        """Perform a real socket connection test to the server's station IP."""
        target_ip = server.station
        target_port = self.port

        # Wait if paused
        await self._pause_event.wait()
        if self._stop_event.is_set():
            return TestResult(
                server_id=server.id,
                name=server.name,
                hostname=server.hostname,
                station=target_ip,
                country_name=server.country_name,
                country_code=server.country_code,
                city=server.city,
                load=server.load,
                port=target_port,
                status=TestStatus.PENDING,
                latency_ms=None,
                tested_at=time.time(),
                is_cached=False,
                error_msg="Cancelled",
            )

        t0 = time.perf_counter()
        status = TestStatus.ONLINE
        latency_ms: Optional[float] = None
        error_msg: Optional[str] = None

        try:
            if self.mode == "tls" and self._ssl_ctx is not None:
                reader, writer = await asyncio.wait_for(
                    asyncio.open_connection(
                        target_ip,
                        target_port,
                        ssl=self._ssl_ctx,
                        server_hostname=server.hostname or None,
                    ),
                    timeout=self.timeout,
                )
            else:
                reader, writer = await asyncio.wait_for(
                    asyncio.open_connection(target_ip, target_port),
                    timeout=self.timeout,
                )

            latency_ms = (time.perf_counter() - t0) * 1000.0
            status = TestStatus.ONLINE

            # Clean socket shutdown without blocking
            try:
                writer.close()
                await asyncio.wait_for(writer.wait_closed(), timeout=0.5)
            except Exception:
                pass

        except asyncio.TimeoutError:
            status = TestStatus.TIMEOUT
            error_msg = f"Timed out (>{self.timeout:.1f}s)"
        except ConnectionRefusedError:
            status = TestStatus.REFUSED
            error_msg = "Connection refused"
        except OSError as e:
            err_str = str(e)
            if "unreachable" in err_str.lower():
                status = TestStatus.UNREACHABLE
            else:
                status = TestStatus.ERROR
            error_msg = err_str
        except Exception as e:
            status = TestStatus.ERROR
            error_msg = str(e)

        result = TestResult(
            server_id=server.id,
            name=server.name,
            hostname=server.hostname,
            station=target_ip,
            country_name=server.country_name,
            country_code=server.country_code,
            city=server.city,
            load=server.load,
            port=target_port,
            status=status,
            latency_ms=round(latency_ms, 2) if latency_ms is not None else None,
            tested_at=time.time(),
            is_cached=False,
            features=server.feature_tags,
            error_msg=error_msg,
        )

        return result

    async def _worker(self, server: ServerInfo):
        """Worker executing with concurrency semaphore."""
        async with self._semaphore:
            if self._stop_event.is_set():
                return
            result = await self._probe_single(server)
            if not self._stop_event.is_set():
                # Enqueue for DB batch writing
                await self._db_queue.put(result)
                # Notify listener
                if self.on_result:
                    self.on_result(result)

    async def _db_writer_worker(self):
        """Dedicated background task for fast batched commits to SQLite."""
        batch: List[TestResult] = []
        sentinel_received = False
        while not sentinel_received:
            try:
                try:
                    item = await asyncio.wait_for(self._db_queue.get(), timeout=0.1)
                except asyncio.TimeoutError:
                    if batch:
                        self.cache_manager.save_batch(batch)
                        batch.clear()
                    if self._stop_event.is_set():
                        break
                    continue

                if item is None:
                    # Sentinel received from scanner.run() finally block
                    sentinel_received = True
                    self._db_queue.task_done()
                    break

                batch.append(item)
                self._db_queue.task_done()

                # Accumulate up to 100 items if immediately available
                while not self._db_queue.empty() and len(batch) < 100:
                    extra = self._db_queue.get_nowait()
                    if extra is None:
                        sentinel_received = True
                        self._db_queue.task_done()
                        break
                    batch.append(extra)
                    self._db_queue.task_done()

                if batch:
                    self.cache_manager.save_batch(batch)
                    batch.clear()

            except asyncio.CancelledError:
                break
            except Exception:
                pass

        if batch:
            try:
                self.cache_manager.save_batch(batch)
            except Exception:
                pass

    async def run(self):
        """Run all server probes with maximum concurrency and persist results."""
        self._stop_event.clear()
        self._pause_event.set()

        # Start background DB writer
        self._db_task = asyncio.create_task(self._db_writer_worker())

        # Spawn all server probe tasks
        tasks = [asyncio.create_task(self._worker(s)) for s in self.servers]

        # Await completion or cancellation
        try:
            await asyncio.gather(*tasks, return_exceptions=True)
        finally:
            # Signal DB worker to flush and terminate
            await self._db_queue.put(None)
            if self._db_task:
                await self._db_task

"""Cross-platform non-blocking keyboard listener for interactive TUI controls."""

import asyncio
import os
import sys
from typing import Optional, Callable


class KeyboardListener:
    """Listens for keyboard presses asynchronously without blocking the event loop."""

    def __init__(self, key_handler: Callable[[str], None]):
        self.key_handler = key_handler
        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._is_windows = os.name == "nt"

    async def start(self):
        """Start listening for keypresses."""
        self._running = True
        self._task = asyncio.create_task(self._listen_loop())

    async def stop(self):
        """Stop listening for keypresses."""
        self._running = False
        if self._task and not self._task.done():
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass

    async def _listen_loop(self):
        """Main polling loop."""
        if not sys.stdin.isatty():
            return

        if self._is_windows:
            await self._listen_windows()
        else:
            await self._listen_posix()

    async def _listen_windows(self):
        import msvcrt

        while self._running:
            try:
                # Check if a key is waiting in buffer
                while msvcrt.kbhit():
                    ch = msvcrt.getch()
                    # Handle multi-byte special keys (e.g. arrow keys 0x00 or 0xe0)
                    if ch in (b"\x00", b"\xe0"):
                        if msvcrt.kbhit():
                            msvcrt.getch()  # consume the scan code
                        continue
                    try:
                        key = ch.decode("utf-8", errors="ignore").lower()
                        if key:
                            self.key_handler(key)
                    except Exception:
                        pass
                await asyncio.sleep(0.05)
            except asyncio.CancelledError:
                break
            except Exception:
                await asyncio.sleep(0.1)

    async def _listen_posix(self):
        import termios
        import tty
        import select

        fd = sys.stdin.fileno()
        old_settings = None
        try:
            old_settings = termios.tcgetattr(fd)
            tty.setcbreak(fd)
        except Exception:
            return

        try:
            while self._running:
                try:
                    rlist, _, _ = select.select([sys.stdin], [], [], 0.05)
                    if rlist:
                        ch = sys.stdin.read(1)
                        if ch:
                            self.key_handler(ch.lower())
                    await asyncio.sleep(0.02)
                except asyncio.CancelledError:
                    break
                except Exception:
                    await asyncio.sleep(0.1)
        finally:
            if old_settings:
                try:
                    termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)
                except Exception:
                    pass

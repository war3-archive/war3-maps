"""Progress reporting for the deploy commands' long passes.

Validating the dataset re-hashes 84 GB and auditing the published URLs makes
ten thousand requests. Both used to run for minutes printing nothing, which is
indistinguishable from a hang — and the operator's only recourse was to check
whether the process still held an open socket.

A terminal gets a line rewritten in place. A pipe or a log file gets one line
per step instead, because a stream of `\\r` is unreadable once captured.

Reports go to stderr, so a command's JSON report on stdout stays pipeable.
"""

from __future__ import annotations

import sys
import time
from collections.abc import Callable, Iterable, Iterator
from concurrent.futures import ThreadPoolExecutor
from typing import TypeVar

T = TypeVar("T")
R = TypeVar("R")

MIB = 1 << 20


class Progress:
    """Count steps towards `total`, reporting no more often than `INTERVAL`."""

    #: Seconds between updates. Fast enough to look live, slow enough that a
    #: redirected run does not produce thousands of lines.
    INTERVAL = 2.0

    def __init__(self, total: int, unit: str, stream=sys.stderr, scale: int = 1) -> None:
        #: Steps are counted in whatever the caller advances by; `scale` divides
        #: them for display, so a byte-counting pass can report MiB.
        self.total = total
        self.unit = unit
        self.stream = stream
        self.scale = scale
        self.tty = stream.isatty()
        self.started = time.monotonic()
        self.last = 0.0
        self.done = 0

    def advance(self, step: int = 1) -> None:
        self.done += step
        now = time.monotonic()
        # The last step always reports, so a finished pass never leaves the
        # line stuck short of its total.
        if now - self.last < self.INTERVAL and self.done < self.total:
            return
        self.last = now
        self.stream.write(self._line() + ("\r" if self.tty else "\n"))
        self.stream.flush()

    def finish(self, note: str) -> None:
        if self.tty:
            # Clear the in-place line so the summary does not land on its tail.
            self.stream.write("\r\033[K")
        self.stream.write(f"{note} in {self.elapsed():.1f}s\n")
        self.stream.flush()

    def elapsed(self) -> float:
        return time.monotonic() - self.started

    def _amount(self, value: int) -> str:
        return f"{value / self.scale:.1f}" if self.scale != 1 else str(value)

    def _line(self) -> str:
        share = self.done / self.total * 100 if self.total else 100.0
        rate = self.done / self.elapsed() if self.elapsed() > 0 else 0.0
        eta = (self.total - self.done) / rate if rate > 0 else 0.0
        return (
            f"  {self._amount(self.done)}/{self._amount(self.total)} {self.unit} "
            f"({share:.0f}%) {self._amount(int(rate))} {self.unit}/s, eta {eta:.0f}s"
        )


def byte_progress(total: int, stream=sys.stderr) -> Progress:
    """A pass counted in bytes, reported in whichever unit reads sensibly.

    A 400 MB rescan in KiB is unreadable; a 40 KB one in MiB reads `0.0/0.0`.
    """
    if total >= 4 * MIB:
        return Progress(total, "MiB", stream=stream, scale=MIB)
    return Progress(total, "KiB", stream=stream, scale=1 << 10)


def track(
    items: Iterable[T],
    unit: str,
    note: str | None = None,
    total: int | None = None,
    stream=sys.stderr,
) -> Iterator[T]:
    """Yield `items`, reporting progress and a closing summary.

    `total` is only needed for iterables that cannot be measured; a list or a
    tuple measures itself.
    """
    if total is None:
        total = len(items)  # type: ignore[arg-type]
    progress = Progress(total, unit, stream=stream)
    for item in items:
        yield item
        progress.advance()
    progress.finish(note or f"processed {progress.done} {unit}")


def map_parallel(
    function: Callable[[T], R],
    items: list[T],
    unit: str,
    workers: int,
    note: str | None = None,
    stream=sys.stderr,
) -> list[R]:
    """Run `function` over `items` in a thread pool, in order, with progress.

    The passes that need this are I/O bound — HEAD requests and image encoding
    — so threads are enough and results stay aligned with their inputs.
    """
    progress = Progress(len(items), unit, stream=stream)
    results: list[R] = []
    with ThreadPoolExecutor(max_workers=workers) as pool:
        for result in pool.map(function, items):
            results.append(result)
            progress.advance()
    progress.finish(note or f"processed {len(items)} {unit}")
    return results

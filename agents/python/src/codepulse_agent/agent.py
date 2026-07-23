"""Runtime agent using sys.monitoring (Python ≥3.12)."""

from __future__ import annotations

import os
import sys
import threading
import time
import traceback
import urllib.error
import urllib.request
import json
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any

PROTOCOL_VERSION = 1
# sys.monitoring tool ids are 0..5 inclusive
TOOL_ID = 3


@dataclass
class Agg:
    invocations: int = 0
    exceptions: int = 0
    durations: list[int] = field(default_factory=list)


class AgentState:
    def __init__(self) -> None:
        self.endpoint = os.environ.get("CODEPULSE_ENDPOINT", "http://127.0.0.1:7420").rstrip("/")
        self.session_id = os.environ.get("CODEPULSE_SESSION_ID") or f"py_{os.getpid()}_{int(time.time())}"
        self.mode = os.environ.get("CODEPULSE_MODE", "baseline")
        self.flush_s = float(os.environ.get("CODEPULSE_FLUSH_S", "2"))
        self.lock = threading.Lock()
        self.stats: dict[tuple[str, str, str], Agg] = defaultdict(Agg)
        self.edges: dict[tuple[tuple[str, str, str], tuple[str, str, str]], int] = defaultdict(int)
        self.starts: dict[int, tuple[float, tuple[str, str, str]]] = {}
        self.targeted: set[tuple[str, str, str]] = set()
        self.events_this_sec = 0
        self.events_sec_bucket = int(time.time())
        self.max_events_per_sec = int(os.environ.get("CODEPULSE_MAX_EVENTS_PER_SEC", "50000"))
        self._stop = threading.Event()
        self.root = os.environ.get("CODEPULSE_ROOT") or os.getcwd()

    def symbol_for(self, code: Any) -> tuple[str, str, str] | None:
        try:
            filename = code.co_filename
            if not filename or filename.startswith("<"):
                return None
            abs_path = os.path.abspath(filename)
            root = os.path.abspath(self.root)
            if not abs_path.startswith(root + os.sep) and abs_path != root:
                return None
            if "site-packages" in abs_path or "/lib/python" in abs_path:
                return None
            path = os.path.relpath(abs_path, root).replace("\\", "/")
            if path.startswith(".."):
                return None
            qual = code.co_qualname if hasattr(code, "co_qualname") else code.co_name
            if qual in {"<module>", "<lambda>"} or qual.startswith("<"):
                return None
            return ("python", path, qual)
        except Exception:
            return None


STATE = AgentState()


def _budget_tick() -> bool:
    now = int(time.time())
    with STATE.lock:
        if now != STATE.events_sec_bucket:
            STATE.events_sec_bucket = now
            STATE.events_this_sec = 0
        STATE.events_this_sec += 1
        if STATE.events_this_sec > STATE.max_events_per_sec:
            STATE.mode = "baseline"
            STATE.targeted.clear()
            return False
        return True


def _on_py_start(code, instruction_offset):  # noqa: ARG001
    try:
        if STATE.mode == "off":
            return
        if not _budget_tick():
            return
        sym = STATE.symbol_for(code)
        if not sym:
            return
        if STATE.mode == "targeted" and sym not in STATE.targeted and STATE.targeted:
            # still count baseline lightly
            pass
        tid = threading.get_ident()
        STATE.starts[tid] = (time.perf_counter(), sym)
    except Exception:
        pass


def _on_py_return(code, instruction_offset, retval):  # noqa: ARG001
    try:
        tid = threading.get_ident()
        start = STATE.starts.pop(tid, None)
        if not start:
            return
        t0, sym = start
        dur = int((time.perf_counter() - t0) * 1_000_000_000)
        caller_sym = None
        try:
            frame = sys._getframe(1)
            while frame is not None:
                cand = STATE.symbol_for(frame.f_code)
                if cand and cand != sym:
                    caller_sym = cand
                    break
                frame = frame.f_back
        except Exception:
            caller_sym = None

        with STATE.lock:
            agg = STATE.stats[sym]
            agg.invocations += 1
            if len(agg.durations) < 64:
                agg.durations.append(dur)
            if caller_sym is not None:
                if STATE.mode != "targeted" or not STATE.targeted or sym in STATE.targeted:
                    STATE.edges[(caller_sym, sym)] += 1
    except Exception:
        pass


def _on_py_raise(code, instruction_offset, exception):  # noqa: ARG001
    try:
        sym = STATE.symbol_for(code)
        if not sym:
            return
        with STATE.lock:
            STATE.stats[sym].exceptions += 1
    except Exception:
        pass


def _percentile(values: list[int], p: float) -> int:
    if not values:
        return 0
    xs = sorted(values)
    idx = int(round((len(xs) - 1) * p))
    return xs[max(0, min(idx, len(xs) - 1))]


def _post_json(path: str, payload: dict) -> Any:
    data = json.dumps(payload).encode()
    req = urllib.request.Request(
        STATE.endpoint + path,
        data=data,
        headers={"content-type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=2) as resp:
        body = resp.read()
        return json.loads(body) if body else None


def _get_json(path: str) -> Any:
    req = urllib.request.Request(STATE.endpoint + path, method="GET")
    with urllib.request.urlopen(req, timeout=2) as resp:
        return json.loads(resp.read().decode())


def _flush_once() -> None:
    end = int(time.time() * 1000)
    start = end - int(STATE.flush_s * 1000)
    with STATE.lock:
        stats_items = list(STATE.stats.items())
        edge_items = list(STATE.edges.items())
        STATE.stats.clear()
        STATE.edges.clear()

    if not stats_items and not edge_items:
        return

    batch = {
        "protocol_version": PROTOCOL_VERSION,
        "session_id": STATE.session_id,
        "process_id": os.getpid(),
        "window_start_ms": start,
        "window_end_ms": end,
        "language": "python",
        "stats": [
            {
                "symbol": {"language": s[0], "path": s[1], "qualname": s[2]},
                "invocations": a.invocations,
                "exceptions": a.exceptions,
                "duration_ns_p50": _percentile(a.durations, 0.5),
                "duration_ns_p95": _percentile(a.durations, 0.95),
            }
            for s, a in stats_items
        ],
        "edges": [
            {
                "caller": {"language": c[0], "path": c[1], "qualname": c[2]},
                "callee": {"language": d[0], "path": d[1], "qualname": d[2]},
                "count": n,
            }
            for (c, d), n in edge_items
        ],
    }
    try:
        data = json.dumps(batch).encode()
        req = urllib.request.Request(
            STATE.endpoint + "/v1/batches",
            data=data,
            headers={"content-type": "application/json"},
            method="POST",
        )
        urllib.request.urlopen(req, timeout=2).read()
    except Exception:
        pass


def _poll_probes() -> None:
    try:
        data = _get_json(f"/v1/probe-commands?session_id={STATE.session_id}")
        for cmd in data.get("commands", []):
            action = cmd.get("action")
            window_id = cmd.get("window_id")
            targets = {
                (t["language"], t["path"], t["qualname"]) for t in cmd.get("targets", [])
            }
            if action == "enable":
                STATE.mode = "targeted"
                STATE.targeted |= targets
                status = "active"
            else:
                STATE.targeted -= targets
                if not STATE.targeted:
                    STATE.mode = "baseline"
                status = "disabled"
            try:
                _post_json(f"/v1/probe-windows/{window_id}/ack", {"status": status})
            except Exception:
                pass
    except Exception:
        pass


def _bg_loop() -> None:
    while not STATE._stop.is_set():
        try:
            _flush_once()
            _poll_probes()
        except Exception:
            traceback.print_exc()
        STATE._stop.wait(STATE.flush_s)


def install() -> str:
    """Install monitoring hooks and background flush. Returns session_id."""
    global TOOL_ID
    chosen: int | None = None
    # Prefer non-reserved ids (0=debugger, 1=coverage, 2=profiler, 3=optimizer)
    for candidate in (4, 5, 3, 2, 1, 0):
        try:
            sys.monitoring.use_tool_id(candidate, "codepulse")
            chosen = candidate
            break
        except ValueError:
            continue
    if chosen is None:
        raise RuntimeError("no free sys.monitoring tool id for codepulse")
    TOOL_ID = chosen

    sys.monitoring.register_callback(TOOL_ID, sys.monitoring.events.PY_START, _on_py_start)
    sys.monitoring.register_callback(TOOL_ID, sys.monitoring.events.PY_RETURN, _on_py_return)
    sys.monitoring.register_callback(TOOL_ID, sys.monitoring.events.RAISE, _on_py_raise)
    sys.monitoring.set_events(
        TOOL_ID,
        sys.monitoring.events.PY_START
        | sys.monitoring.events.PY_RETURN
        | sys.monitoring.events.RAISE,
    )
    sys.monitoring.restart_events()
    print(f"codepulse agent session={STATE.session_id} tool_id={TOOL_ID} root={STATE.root}", flush=True)
    t = threading.Thread(target=_bg_loop, name="codepulse-agent", daemon=True)
    t.start()
    return STATE.session_id


def shutdown() -> None:
    STATE._stop.set()
    try:
        _flush_once()
    except Exception:
        pass
    try:
        sys.monitoring.set_events(TOOL_ID, 0)
    except Exception:
        pass

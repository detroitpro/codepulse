# codepulse Python agent

In-process CPython agent using `sys.monitoring` (Python ≥3.12).

```bash
export CODEPULSE_ENDPOINT=http://127.0.0.1:7420
export CODEPULSE_ROOT=/path/to/workspace
python -m codepulse_agent run app.py
# or
python -m codepulse_agent run -m uvicorn examples.python_demo.app:app -- --port 8000
```

Environment:

| Variable | Meaning |
|---|---|
| `CODEPULSE_ENDPOINT` | Daemon base URL |
| `CODEPULSE_SESSION_ID` | Optional stable session id |
| `CODEPULSE_ROOT` | Workspace root for path normalization |
| `CODEPULSE_MODE` | `baseline` / `targeted` / `off` |

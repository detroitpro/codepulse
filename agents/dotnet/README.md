# codepulse .NET agent

In-process agent using [Lib.Harmony](https://github.com/pardeike/Harmony) to patch methods in namespaces matching `CODEPULSE_INCLUDE`.

```bash
export CODEPULSE_ENDPOINT=http://127.0.0.1:7420
export CODEPULSE_INCLUDE=DotnetDemo
export CODEPULSE_SESSION_ID=my_session

# From a host app:
# CodePulseAgent.Install("DotnetDemo");
```

See `examples/dotnet-demo` and `scripts/e2e-dotnet.sh`.

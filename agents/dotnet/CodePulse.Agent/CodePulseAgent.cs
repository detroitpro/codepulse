using System.Collections.Concurrent;
using System.Diagnostics;
using System.Net.Http.Json;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;
using HarmonyLib;

namespace CodePulse.Agent;

public static class CodePulseAgent
{
    private static readonly ConcurrentDictionary<string, Agg> Stats = new();
    private static readonly ConcurrentDictionary<string, long> Edges = new();
    private static readonly HashSet<string> Targeted = new();
    private static readonly object Gate = new();

    private static HttpClient? _http;
    private static string _endpoint = "http://127.0.0.1:7420";
    private static string _sessionId = "";
    private static string _root = "";
    private static string _include = "";
    private static string _mode = "baseline";
    private static CancellationTokenSource? _cts;
    private static Harmony? _harmony;
    private static long _eventsThisSec;
    private static long _secBucket;

    public static string Install(string? includePrefix = null)
    {
        _endpoint = (Environment.GetEnvironmentVariable("CODEPULSE_ENDPOINT") ?? "http://127.0.0.1:7420").TrimEnd('/');
        _sessionId = Environment.GetEnvironmentVariable("CODEPULSE_SESSION_ID")
                     ?? $"dotnet_{Environment.ProcessId}_{DateTimeOffset.UtcNow.ToUnixTimeSeconds()}";
        _root = Environment.GetEnvironmentVariable("CODEPULSE_ROOT") ?? Directory.GetCurrentDirectory();
        _include = includePrefix
                   ?? Environment.GetEnvironmentVariable("CODEPULSE_INCLUDE")
                   ?? "DotnetDemo";
        _mode = Environment.GetEnvironmentVariable("CODEPULSE_MODE") ?? "baseline";
        _http = new HttpClient { Timeout = TimeSpan.FromSeconds(2) };
        _cts = new CancellationTokenSource();

        _harmony = new Harmony("codepulse.agent");
        var patched = PatchAssemblies(_harmony);
        Console.WriteLine($"codepulse patched {patched} methods include={_include} endpoint={_endpoint}");
        Console.Out.Flush();

        _ = Task.Run(() => LoopAsync(_cts.Token));
        return _sessionId;
    }

    private static bool NamespaceMatches(string ns, string include)
    {
        var prefix = include.Trim().TrimEnd('.');
        return ns.Equals(prefix, StringComparison.Ordinal)
               || ns.StartsWith(prefix + ".", StringComparison.Ordinal);
    }

    private static int PatchAssemblies(Harmony harmony)
    {
        var count = 0;
        foreach (var asm in AppDomain.CurrentDomain.GetAssemblies())
        {
            Type[] types;
            try { types = asm.GetTypes(); }
            catch (ReflectionTypeLoadException ex)
            {
                types = ex.Types.Where(t => t != null).Cast<Type>().ToArray();
            }
            catch { continue; }

            foreach (var type in types)
            {
                if (type.IsSpecialName) continue;
                var ns = type.Namespace ?? "";
                // Also allow type full-name prefix matches for file-local / nested quirks
                var matches = NamespaceMatches(ns, _include)
                              || type.FullName?.StartsWith(_include.TrimEnd('.'), StringComparison.Ordinal) == true;
                if (!matches) continue;

                var methods = type.GetMethods(
                    BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public |
                    BindingFlags.NonPublic | BindingFlags.DeclaredOnly);
                foreach (var method in methods)
                {
                    if (method.IsAbstract || method.ContainsGenericParameters)
                        continue;
                    if (method.Name is "Equals" or "GetHashCode" or "ToString" or "GetType")
                        continue;
                    if (method.Name.StartsWith("get_") || method.Name.StartsWith("set_") || method.Name.StartsWith("<"))
                        continue;
                    if (type.Name.Contains("CheckoutRequest") || type.Name.Contains("d__"))
                        continue;
                    try
                    {
                        var prefix = new HarmonyMethod(typeof(Hooks), nameof(Hooks.Prefix));
                        var postfix = new HarmonyMethod(typeof(Hooks), nameof(Hooks.Postfix));
                        harmony.Patch(method, prefix: prefix, postfix: postfix);
                        count++;
                    }
                    catch
                    {
                        // never crash host
                    }
                }
            }
        }
        return count;
    }

    internal static bool BudgetOk()
    {
        var sec = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        if (sec != Interlocked.Read(ref _secBucket))
        {
            Interlocked.Exchange(ref _secBucket, sec);
            Interlocked.Exchange(ref _eventsThisSec, 0);
        }
        var n = Interlocked.Increment(ref _eventsThisSec);
        if (n > 50_000)
        {
            lock (Gate)
            {
                _mode = "baseline";
                Targeted.Clear();
            }
            return false;
        }
        return true;
    }

    public static string SymbolKey(MethodBase method)
    {
        var type = method.DeclaringType;
        var ns = type?.Namespace ?? "";
        var typeName = type?.Name ?? "Unknown";
        var qual = string.IsNullOrEmpty(ns) ? $"{typeName}.{method.Name}" : $"{ns}.{typeName}.{method.Name}";
        var path = "Program.cs";
        return $"csharp|{path}|{qual}";
    }

    internal static void RecordEnd(MethodBase method, long startTs, string symbolKey, Exception? ex)
    {
        try
        {
            var elapsed = (long)((Stopwatch.GetTimestamp() - startTs) * 1_000_000_000.0 / Stopwatch.Frequency);
            var agg = Stats.GetOrAdd(symbolKey, _ => new Agg());
            Interlocked.Increment(ref agg.Invocations);
            if (ex != null) Interlocked.Increment(ref agg.Exceptions);
            agg.AddDuration(elapsed);

            bool targeted;
            lock (Gate)
            {
                targeted = _mode == "targeted" && (Targeted.Count == 0 || Targeted.Contains(symbolKey));
            }
            if (targeted)
            {
                var st = new StackTrace(2, false);
                var frame = st.GetFrame(0);
                var caller = frame?.GetMethod();
                if (caller != null)
                {
                    var edgeKey = SymbolKey(caller) + "->" + symbolKey;
                    Edges.AddOrUpdate(edgeKey, 1, (_, v) => v + 1);
                }
            }
        }
        catch
        {
            // swallow
        }
    }

    private static async Task LoopAsync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                await FlushAsync(ct);
                await PollProbesAsync(ct);
            }
            catch
            {
                // never crash
            }
            try { await Task.Delay(TimeSpan.FromSeconds(2), ct); }
            catch { break; }
        }
    }

    private static async Task FlushAsync(CancellationToken ct)
    {
        if (_http == null) return;
        var end = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var start = end - 2000;

        var statsSnap = Stats.ToArray();
        foreach (var kv in statsSnap) Stats.TryRemove(kv.Key, out _);
        var edgeSnap = Edges.ToArray();
        foreach (var kv in edgeSnap) Edges.TryRemove(kv.Key, out _);

        if (statsSnap.Length == 0 && edgeSnap.Length == 0) return;

        var batch = new RuntimeStatBatch
        {
            ProtocolVersion = 1,
            SessionId = _sessionId,
            ProcessId = (uint)Environment.ProcessId,
            WindowStartMs = (ulong)start,
            WindowEndMs = (ulong)end,
            Language = "csharp",
            Stats = statsSnap.Select(kv =>
            {
                var parts = kv.Key.Split('|');
                return new FunctionRuntimeStat
                {
                    Symbol = new SymbolId { Language = parts[0], Path = parts[1], Qualname = parts[2] },
                    Invocations = (ulong)Math.Max(0, kv.Value.Invocations),
                    Exceptions = (ulong)Math.Max(0, kv.Value.Exceptions),
                    DurationNsP50 = (ulong)kv.Value.Percentile(0.5),
                    DurationNsP95 = (ulong)kv.Value.Percentile(0.95),
                };
            }).ToList(),
            Edges = edgeSnap.Select(kv =>
            {
                var sides = kv.Key.Split(new[] { "->" }, StringSplitOptions.None);
                var c = sides[0].Split('|');
                var d = sides[1].Split('|');
                return new CallEdge
                {
                    Caller = new SymbolId { Language = c[0], Path = c[1], Qualname = c[2] },
                    Callee = new SymbolId { Language = d[0], Path = d[1], Qualname = d[2] },
                    Count = (ulong)kv.Value,
                };
            }).ToList(),
        };

        try
        {
            await _http.PostAsJsonAsync($"{_endpoint}/v1/batches", batch, ct);
        }
        catch { /* drop */ }
    }

    private static async Task PollProbesAsync(CancellationToken ct)
    {
        if (_http == null) return;
        try
        {
            var resp = await _http.GetFromJsonAsync<ProbeCommandsResponse>(
                $"{_endpoint}/v1/probe-commands?session_id={Uri.EscapeDataString(_sessionId)}", ct);
            if (resp?.Commands == null) return;
            foreach (var cmd in resp.Commands)
            {
                var keys = cmd.Targets.Select(t => $"csharp|{t.Path}|{t.Qualname}").ToHashSet();
                // also accept whatever language the MCP sent
                foreach (var t in cmd.Targets)
                    keys.Add($"{t.Language}|{t.Path}|{t.Qualname}");

                string status;
                lock (Gate)
                {
                    if (cmd.Action == "enable")
                    {
                        _mode = "targeted";
                        foreach (var k in keys) Targeted.Add(k);
                        status = "active";
                    }
                    else
                    {
                        foreach (var k in keys) Targeted.Remove(k);
                        if (Targeted.Count == 0) _mode = "baseline";
                        status = "disabled";
                    }
                }
                try
                {
                    await _http.PostAsJsonAsync(
                        $"{_endpoint}/v1/probe-windows/{cmd.WindowId}/ack",
                        new { status }, ct);
                }
                catch { /* ignore */ }
            }
        }
        catch { /* ignore */ }
    }

    private sealed class Agg
    {
        public long Invocations;
        public long Exceptions;
        private readonly List<long> _durations = new();
        private readonly object _lock = new();
        public void AddDuration(long ns)
        {
            lock (_lock)
            {
                if (_durations.Count < 64) _durations.Add(ns);
            }
        }
        public long Percentile(double p)
        {
            lock (_lock)
            {
                if (_durations.Count == 0) return 0;
                var xs = _durations.OrderBy(x => x).ToList();
                var idx = (int)Math.Round((xs.Count - 1) * p);
                idx = Math.Clamp(idx, 0, xs.Count - 1);
                return xs[idx];
            }
        }
    }
}

internal static class Hooks
{
    public static void Prefix(MethodBase __originalMethod, ref object? __state)
    {
        __state = null;
        try
        {
            if (!CodePulseAgent.BudgetOk()) return;
            __state = (Stopwatch.GetTimestamp(), CodePulseAgent.SymbolKey(__originalMethod));
        }
        catch { /* ignore */ }
    }

    public static void Postfix(MethodBase __originalMethod, object? __state)
    {
        try
        {
            if (__state is not ValueTuple<long, string> st || st.Item1 == 0) return;
            CodePulseAgent.RecordEnd(__originalMethod, st.Item1, st.Item2, null);
        }
        catch { /* ignore */ }
    }
}

internal sealed class RuntimeStatBatch
{
    [JsonPropertyName("protocol_version")] public uint ProtocolVersion { get; set; }
    [JsonPropertyName("session_id")] public string SessionId { get; set; } = "";
    [JsonPropertyName("process_id")] public uint ProcessId { get; set; }
    [JsonPropertyName("window_start_ms")] public ulong WindowStartMs { get; set; }
    [JsonPropertyName("window_end_ms")] public ulong WindowEndMs { get; set; }
    [JsonPropertyName("language")] public string? Language { get; set; }
    [JsonPropertyName("stats")] public List<FunctionRuntimeStat> Stats { get; set; } = new();
    [JsonPropertyName("edges")] public List<CallEdge> Edges { get; set; } = new();
}

internal sealed class FunctionRuntimeStat
{
    [JsonPropertyName("symbol")] public SymbolId Symbol { get; set; } = new();
    [JsonPropertyName("invocations")] public ulong Invocations { get; set; }
    [JsonPropertyName("exceptions")] public ulong Exceptions { get; set; }
    [JsonPropertyName("duration_ns_p50")] public ulong DurationNsP50 { get; set; }
    [JsonPropertyName("duration_ns_p95")] public ulong DurationNsP95 { get; set; }
}

internal sealed class CallEdge
{
    [JsonPropertyName("caller")] public SymbolId Caller { get; set; } = new();
    [JsonPropertyName("callee")] public SymbolId Callee { get; set; } = new();
    [JsonPropertyName("count")] public ulong Count { get; set; }
}

internal sealed class SymbolId
{
    [JsonPropertyName("language")] public string Language { get; set; } = "";
    [JsonPropertyName("path")] public string Path { get; set; } = "";
    [JsonPropertyName("qualname")] public string Qualname { get; set; } = "";
}

internal sealed class ProbeCommandsResponse
{
    [JsonPropertyName("commands")] public List<ProbeCommandDto> Commands { get; set; } = new();
}

internal sealed class ProbeCommandDto
{
    [JsonPropertyName("window_id")] public string WindowId { get; set; } = "";
    [JsonPropertyName("action")] public string Action { get; set; } = "";
    [JsonPropertyName("targets")] public List<SymbolId> Targets { get; set; } = new();
}

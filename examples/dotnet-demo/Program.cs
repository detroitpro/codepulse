using CodePulse.Agent;
using DotnetDemo;

var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

// Force-load demo types before patching
_ = typeof(PricingWorkflow);
_ = typeof(Inventory);
Environment.SetEnvironmentVariable("CODEPULSE_INCLUDE", "DotnetDemo");
var session = CodePulseAgent.Install("DotnetDemo");
Console.WriteLine($"codepulse session={session}");

app.MapGet("/health", () => Results.Ok(new { ok = true }));
app.MapPost("/checkout", (CheckoutRequest req) =>
{
    var total = PricingWorkflow.Execute(req.Total);
    Inventory.Reserve(req.Sku ?? "SKU-1", 1);
    return Results.Ok(new { total, sku = req.Sku ?? "SKU-1" });
});

app.Run("http://127.0.0.1:8010");

namespace DotnetDemo
{
    public record CheckoutRequest(double Total, string? Sku);

    public static class PricingWorkflow
    {
        public static double Execute(double cartTotal)
        {
            Thread.Sleep(1);
            return Math.Round(cartTotal * 1.08, 2);
        }
    }

    public static class Inventory
    {
        public static bool Reserve(string sku, int qty)
        {
            Thread.Sleep(1);
            return qty > 0 && !string.IsNullOrEmpty(sku);
        }
    }

    public static class Unused
    {
        public static async Task<int> UnusedAsyncHelper()
        {
            await Task.Yield();
            return 1;
        }
    }
}

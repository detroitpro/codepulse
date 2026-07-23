using CodePulse.Agent;

namespace CodePulse.Agent.Tests;

public class UnitTest1
{
    [Fact]
    public void SymbolKey_UsesNamespaceTypeMethod()
    {
        var method = typeof(Sample).GetMethod(nameof(Sample.DoWork))!;
        var key = CodePulseAgent.SymbolKey(method);
        Assert.Contains("CodePulse.Agent.Tests.Sample.DoWork", key);
        Assert.StartsWith("csharp|", key);
    }

    public class Sample
    {
        public void DoWork() { }
    }
}

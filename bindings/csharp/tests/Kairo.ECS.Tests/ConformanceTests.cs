using System.Text.Json;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace Kairo.ECS.Tests;

[TestClass]
public sealed class ConformanceTests
{
    private static readonly string FixtureRoot = FindFixtureRoot();

    [TestMethod]
    public void DeterministicOrdering_Fixture_IsValid()
    {
        var json = File.ReadAllText(Path.Combine(FixtureRoot, "deterministic_ordering.json"));
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;
        Assert.AreEqual(1, root.GetProperty("version").GetInt32());
        var kinds = root.GetProperty("expected_kind_order").EnumerateArray()
            .Select(e => e.GetInt32()).ToArray();
        CollectionAssert.AreEqual(new[] { 1, 2, 4, 3 }, kinds);
    }

    [TestMethod]
    public void Cancellation_Fixture_IsValid()
    {
        var json = File.ReadAllText(Path.Combine(FixtureRoot, "cancellation.json"));
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;
        var kinds = root.GetProperty("expected_kind_order").EnumerateArray()
            .Select(e => e.GetInt32()).ToArray();
        CollectionAssert.AreEqual(new[] { 1, 3 }, kinds);
    }

    [TestMethod]
    public void RngReplay_Fixture_IsValid()
    {
        var json = File.ReadAllText(Path.Combine(FixtureRoot, "rng_replay.json"));
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;
        Assert.AreEqual(7, root.GetProperty("run_seed").GetInt32());
        var stream = root.GetProperty("expected_stream").EnumerateArray().ToArray();
        Assert.AreEqual(4, stream.Length);
    }

    private static string FindFixtureRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null)
        {
            var candidate = Path.Combine(directory.FullName, "conformance", "fixtures");
            if (Directory.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        throw new DirectoryNotFoundException("Could not locate conformance/fixtures from the test output directory.");
    }
}

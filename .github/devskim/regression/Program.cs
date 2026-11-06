using System.Diagnostics;
using System.Text.Json.Nodes;

if (args.Length != 2) throw new ArgumentException("Expected upstream and maintained tool directories");
string directory = Path.Combine(Path.GetTempPath(), "devskim-maintenance-" + Guid.NewGuid());
Directory.CreateDirectory(directory);
try
{
    // Generate the diagnostic sample at build time; these are synthetic strings.
    string first = string.Concat(Enumerable.Repeat("1234567890abcdef", 2));
    string second = string.Concat(Enumerable.Repeat("abcdef1234567890", 2));
    string url = "http" + "://example.com";
    string source = Path.Combine(directory, "sample.cs");
    File.WriteAllLines(source, new[]
    {
        "// Synthetic regression input for the performance-only DevSkim patch.",
        $"[assembly: InternalsVisibleTo(\"Tests, PublicKey={first}\")]",
        $"var digest = \"{first}\"; var other = \"{second}\";",
        $"var suppressed = \"{first}\"; // DevSkim: ignore DS173237 until 2099-01-01",
        $"var expired = \"{second}\"; // DevSkim: ignore DS173237 until 2000-01-01",
        $"var ordinary = \"{first}\";",
        $"var url = \"{url}\"; // DevSkim: ignore DS137138 until 2099-01-01",
        $"var url2 = \"{url}\";"
    });
    var original = await Scan(args[0], source, Path.Combine(directory, "original.sarif"));
    var maintained = await Scan(args[1], source, Path.Combine(directory, "maintained.sarif"));
    Validate(original);
    Validate(maintained);
    if (!JsonNode.DeepEquals(Canonical(original), Canonical(maintained)))
        throw new InvalidOperationException("Maintained scanner changed synthetic SARIF content");
    Console.WriteLine("DEVSKIM_MAINTENANCE_REGRESSION_PASS results=5 full_sarif_equal=true");
}
finally
{
    Directory.Delete(directory, recursive: true);
}

static void Validate(JsonNode log)
{
    var run = log["runs"]![0]!;
    if (run["tool"]!["driver"]!["version"]!.GetValue<string>() != "1.0.90+fb2d676ce4")
        throw new InvalidOperationException("Unexpected upstream scanner identity");
    var results = run["results"]!.AsArray();
    if (results.Count != 5 || results.Any(result => result?["level"] is null))
        throw new InvalidOperationException("Missing findings or default warning levels");
    var lines = results.Select(result => result!["locations"]![0]!["physicalLocation"]!["region"]!["startLine"]!.GetValue<int>()).Order().ToArray();
    if (!lines.SequenceEqual(new[] { 3, 3, 5, 6, 8 }))
        throw new InvalidOperationException("same-line or suppression semantics changed");
}

static JsonNode? Canonical(JsonNode? value)
{
    if (value is JsonObject obj)
    {
        var sorted = new JsonObject();
        foreach (var property in obj.OrderBy(pair => pair.Key, StringComparer.Ordinal))
            sorted[property.Key] = Canonical(property.Value);
        return sorted;
    }
    if (value is JsonArray array)
    {
        var items = array.Select(Canonical).ToArray();
        if (array.GetPropertyName() == "results")
            items = items.OrderBy(item => item!.ToJsonString(), StringComparer.Ordinal).ToArray();
        return new JsonArray(items);
    }
    return value?.DeepClone();
}

static async Task<JsonNode> Scan(string tools, string source, string output)
{
    string dll = Path.Combine(tools, ".store/microsoft.cst.devskim.cli/1.0.90/microsoft.cst.devskim.cli/1.0.90/tools/net8.0/any/devskim.dll");
    var start = new ProcessStartInfo("dotnet") { RedirectStandardOutput = true, RedirectStandardError = true };
    foreach (string argument in new[] { dll, "analyze", "--source-code", source, "--output-file", output })
        start.ArgumentList.Add(argument);
    using var process = Process.Start(start) ?? throw new InvalidOperationException("Scanner did not start");
    var stdout = process.StandardOutput.ReadToEndAsync();
    var stderr = process.StandardError.ReadToEndAsync();
    using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(90));
    try
    {
        await process.WaitForExitAsync(deadline.Token);
    }
    catch (OperationCanceledException)
    {
        process.Kill(entireProcessTree: true);
        await process.WaitForExitAsync();
        throw new InvalidOperationException("Synthetic scan exceeded 90 seconds");
    }
    await stdout;
    string error = await stderr;
    if (process.ExitCode != 0) throw new InvalidOperationException($"Scanner exited {process.ExitCode}: {error}");
    return JsonNode.Parse(await File.ReadAllTextAsync(output)) ?? throw new InvalidOperationException("Empty SARIF");
}

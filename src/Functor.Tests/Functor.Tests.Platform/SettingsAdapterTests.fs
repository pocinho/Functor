namespace Functor.Tests.Platform

open System
open System.IO
open Functor.Platform
open Xunit

type SettingsAdapterTests() =
    [<Fact>]
    member _.``writes settings atomically and preserves the previous file``() =
        let directory = Path.Combine(Path.GetTempPath(), "FunctorSettingsTests", Guid.NewGuid().ToString("N"))
        let path = Path.Combine(directory, "settings.json")

        try
            Assert.Equal(Ok(), Settings.tryWriteText path "first")
            Assert.Equal("first", File.ReadAllText(path))

            Assert.Equal(Ok(), Settings.tryWriteText path "second")
            Assert.Equal("second", File.ReadAllText(path))
            Assert.True(File.Exists(path + ".bak"))
            Assert.Equal("first", File.ReadAllText(path + ".bak"))
        finally
            if Directory.Exists(directory) then
                Directory.Delete(directory, true)

open System
open System.Diagnostics
open System.IO

let repositoryRoot = DirectoryInfo(__SOURCE_DIRECTORY__).Parent.FullName
let buildScript = Path.Combine(__SOURCE_DIRECTORY__, "build.fsx")
let outputRoot = Path.Combine(repositoryRoot, "out")
let outputDirectory = Path.Combine(outputRoot, "win-x64")

let arguments =
    sprintf
        "fsi \"%s\" --runtimeIdentifier win-x64 --outputRoot \"%s\" --outputDirectory \"%s\" --configuration Release"
        buildScript
        outputRoot
        outputDirectory

let buildProcess = new Process()
buildProcess.StartInfo <- ProcessStartInfo("dotnet", arguments)
buildProcess.StartInfo.WorkingDirectory <- repositoryRoot
buildProcess.StartInfo.UseShellExecute <- false

if not (buildProcess.Start()) then
    failwith "Could not start build.fsx."

buildProcess.WaitForExit()

if buildProcess.ExitCode <> 0 then
    failwithf "build.fsx failed with exit code %d." buildProcess.ExitCode

buildProcess.Dispose()

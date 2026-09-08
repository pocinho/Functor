open System
open System.Diagnostics
open System.IO

let buildDirectory = __SOURCE_DIRECTORY__
let repositoryRoot = DirectoryInfo(buildDirectory).Parent.FullName

let project =
    Path.Combine(repositoryRoot, "src", "Functor.App.Desktop", "Functor.App.Desktop.fsproj")

let resolvePath (path: string) =
    if Path.IsPathRooted(path) then
        path
    else
        Path.GetFullPath(Path.Combine(repositoryRoot, path))

let parseArguments () =
    let arguments = fsi.CommandLineArgs |> Array.skip 1

    let rec parse (remaining: string list) (values: Map<string, string>) =
        match remaining with
        | [] -> values
        | optionName :: optionValue :: tail when optionName.StartsWith("--") ->
            parse tail (Map.add (optionName.Substring(2)) optionValue values)
        | [ optionName ] when optionName = "--help" ->
            Console.WriteLine(
                "Usage: dotnet fsi build.fsx [--runtimeIdentifier VALUE] [--outputRoot VALUE] [--outputDirectory VALUE] [--configuration VALUE]"
            )

            exit 0
        | optionName :: _ -> failwithf "Invalid command-line arguments near '%s'. Use --help for usage." optionName

    let values = parse (Array.toList arguments) Map.empty

    let get name defaultValue =
        Map.tryFind name values |> Option.defaultValue defaultValue

    let runtimeIdentifier = get "runtimeIdentifier" "win-x64"

    let outputRoot =
        get "outputRoot" (Path.Combine(repositoryRoot, "out")) |> resolvePath

    let outputDirectory =
        get "outputDirectory" (Path.Combine(outputRoot, runtimeIdentifier))
        |> resolvePath

    let configuration = get "configuration" "Release"

    runtimeIdentifier, outputRoot, outputDirectory, configuration

let runtimeIdentifier, outputRoot, outputDirectory, configuration =
    parseArguments ()

let run (command: string) (arguments: string) (workingDirectory: string) =
    let startInfo = ProcessStartInfo(command, arguments)
    startInfo.WorkingDirectory <- workingDirectory
    startInfo.UseShellExecute <- false
    startInfo.RedirectStandardOutput <- true
    startInfo.RedirectStandardError <- true

    use childProcess = new Process()
    childProcess.StartInfo <- startInfo

    childProcess.OutputDataReceived.Add(fun eventArgs ->
        if not (isNull eventArgs.Data) then
            Console.WriteLine(eventArgs.Data))

    childProcess.ErrorDataReceived.Add(fun eventArgs ->
        if not (isNull eventArgs.Data) then
            Console.Error.WriteLine(eventArgs.Data))

    if not (childProcess.Start()) then
        failwithf "Could not start %s." command

    childProcess.BeginOutputReadLine()
    childProcess.BeginErrorReadLine()
    childProcess.WaitForExit()

    if childProcess.ExitCode <> 0 then
        failwithf "%s failed with exit code %d." command childProcess.ExitCode

let clean () =
    if Directory.Exists(outputRoot) then
        Directory.Delete(outputRoot, true)

let publish () =
    run
        "dotnet"
        (sprintf
            "publish \"%s\" --configuration %s --runtime %s --no-self-contained --output \"%s\""
            project
            configuration
            runtimeIdentifier
            outputDirectory)
        repositoryRoot

let signAndVerify () =
    let certificatePath = Path.Combine(buildDirectory, "FunctorDev.pfx")

    let certificatePassword =
        Environment.GetEnvironmentVariable("FUNCTOR_CERT_PASSWORD")

    let signTool = @"C:\Program Files (x86)\Windows Kits\10\bin\x64\signtool.exe"

    if
        String.IsNullOrWhiteSpace(certificatePassword)
        || not (File.Exists(certificatePath))
        || not (File.Exists(signTool))
    then
        Console.WriteLine("Signing skipped: certificate, password, or signtool is unavailable.")
    else
        for file in Directory.EnumerateFiles(outputDirectory, "*.exe", SearchOption.AllDirectories) do
            run
                signTool
                (sprintf "sign /f \"%s\" /p \"%s\" /fd SHA256 /v \"%s\"" certificatePath certificatePassword file)
                repositoryRoot

            run signTool (sprintf "verify /pa \"%s\"" file) repositoryRoot

clean ()
publish ()
signAndVerify ()

Console.WriteLine(sprintf "Build completed. Output: %s" outputDirectory)

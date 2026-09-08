open System
open System.Diagnostics
open System.IO

let buildDirectory = __SOURCE_DIRECTORY__
let repositoryRoot = DirectoryInfo(buildDirectory).Parent.FullName

let project =
    Path.Combine(repositoryRoot, "src", "Functor.App.Desktop", "Functor.App.Desktop.fsproj")

let outputDirectory = Path.Combine(repositoryRoot, "out")
let configuration = "Debug"
let targetFramework = "net10.0"

let projectOutputDirectory =
    Path.Combine(repositoryRoot, "src", "Functor.App.Desktop", "bin", configuration, targetFramework)

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
    if Directory.Exists(outputDirectory) then
        Directory.Delete(outputDirectory, true)

let build () =
    run "dotnet" (sprintf "build \"%s\" --configuration %s" project configuration) repositoryRoot

let copyOutput () =
    if not (Directory.Exists(projectOutputDirectory)) then
        failwithf "Build output directory does not exist: %s" projectOutputDirectory

    Directory.CreateDirectory(outputDirectory) |> ignore

    for sourceFile in Directory.EnumerateFiles(projectOutputDirectory, "*", SearchOption.AllDirectories) do
        let relativePath = Path.GetRelativePath(projectOutputDirectory, sourceFile)
        let destinationFile = Path.Combine(outputDirectory, relativePath)
        Directory.CreateDirectory(Path.GetDirectoryName(destinationFile)) |> ignore
        File.Copy(sourceFile, destinationFile, true)

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
build ()
copyOutput ()
signAndVerify ()

Console.WriteLine(sprintf "Build completed. Output: %s" outputDirectory)

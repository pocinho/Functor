namespace Functor.Tests.Architecture

open System
open System.IO
open System.Xml.Linq
open Xunit

module ProjectReferenceRulesTests =
    let private projectRoot =
        let rec findRoot (directory: DirectoryInfo) =
            if File.Exists(Path.Combine(directory.FullName, "src", "Functor.slnx")) then
                directory.FullName
            elif isNull directory.Parent then
                failwith "Could not locate the repository root."
            else
                findRoot directory.Parent

        findRoot (DirectoryInfo(AppContext.BaseDirectory))

    let private projectReferences projectName =
        let projectPath =
            Directory.GetFiles(Path.Combine(projectRoot, "src"), projectName + ".fsproj", SearchOption.AllDirectories)
            |> Array.tryHead
            |> Option.defaultWith (fun () -> failwithf "Could not locate project %s." projectName)

        XDocument.Load(projectPath).Descendants(XName.Get "ProjectReference")
        |> Seq.choose (fun reference -> reference.Attribute(XName.Get "Include") |> Option.ofObj)
        |> Seq.map (fun attribute -> Path.GetFileNameWithoutExtension(attribute.Value))
        |> Set.ofSeq

    [<Fact>]
    let ``domain has no project references`` () =
        Assert.Empty(projectReferences "Functor.Domain")

    [<Fact>]
    let ``workspace and rendering reference domain only`` () =
        Assert.True(Set [ "Functor.Domain" ] = projectReferences "Functor.Workspace")
        Assert.True(Set [ "Functor.Domain" ] = projectReferences "Functor.Rendering")

    [<Fact>]
    let ``application references only approved inner projects`` () =
        let references = projectReferences "Functor.Application"
        let referencesText = String.Join(", ", references)
        let forbidden =
            Set.intersect references (Set [ "Functor.Avalonia"; "Functor.Platform"; "Functor.Lsp"; "Functor.PluginHost"; "Functor.Agent" ])

        Assert.Empty(forbidden)
        Assert.True(
            references.IsSubsetOf(Set [ "Functor.Domain"; "Functor.Workspace"; "Functor.Rendering" ]),
            $"Application has unexpected project references: {referencesText}")

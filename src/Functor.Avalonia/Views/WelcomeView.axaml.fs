namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Markup.Xaml

type WelcomeView() as this =
    inherit UserControl()

    let newFileButton = lazy (this.FindControl<Button>("NewFileButton"))
    let openFileButton = lazy (this.FindControl<Button>("OpenFileButton"))
    let openFolderButton = lazy (this.FindControl<Button>("OpenFolderButton"))

    do this.InitializeComponent()

    member _.NewFileRequested = newFileButton.Value.Click

    member _.OpenFileRequested = openFileButton.Value.Click

    member _.OpenFolderRequested = openFolderButton.Value.Click

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)

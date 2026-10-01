namespace Functor.Application

type ShellCommandResult =
    | CommandFound of AppCommandDescriptor
    | CommandNotFound of commandId: string

module ShellCommands =
    let all = AppCommandCatalog.all

    let tryFindById id =
        AppCommandCatalog.tryFindById id

    let resolveById id =
        match tryFindById id with
        | Some command -> CommandFound command
        | None -> CommandNotFound id

    let filter query commands =
        AppCommandCatalog.filter query commands

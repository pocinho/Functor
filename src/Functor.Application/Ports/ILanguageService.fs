namespace Functor.Application

open System
open System.Threading
open System.Threading.Tasks
open Functor.Domain.Diagnostics
open Functor.Domain.Document

type LanguageDocument =
    { DocumentId: DocumentId
      Path: string option
      Version: int64
      Lines: string list }

type LanguageServiceFailure =
    | NotInitialized
    | UnsupportedLanguage of language: string
    | RequestCancelled
    | StaleResult
    | TransportFailure of message: string
    | ProtocolFailure of message: string

type ILanguageService =
    abstract StartAsync: language: string * cancellationToken: CancellationToken -> Task<Result<unit, LanguageServiceFailure>>
    abstract PublishDocumentAsync: document: LanguageDocument * cancellationToken: CancellationToken -> Task<Result<unit, LanguageServiceFailure>>
    abstract RequestDiagnosticsAsync: document: LanguageDocument * cancellationToken: CancellationToken -> Task<Result<Diagnostic list, LanguageServiceFailure>>
    abstract StopAsync: cancellationToken: CancellationToken -> Task<Result<unit, LanguageServiceFailure>>
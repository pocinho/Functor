namespace Functor.Lsp

open System
open System.Threading
open System.Threading.Tasks
open Functor.Application

type LspPosition =
    { Line: int
      Character: int }

type LspRange =
    { Start: LspPosition
      End: LspPosition }

type LspDiagnosticSeverity =
    | Error
    | Warning
    | Information
    | Hint

type LspDiagnostic =
    { Range: LspRange
      Severity: LspDiagnosticSeverity
      Message: string
      Code: string option
      Source: string option }

type LspRequest =
    | StartLanguage of language: string
    | PublishDocument of document: LanguageDocument
    | RequestDiagnostics of document: LanguageDocument
    | StopLanguage

type LspResponse =
    | LanguageStarted
    | DocumentPublished
    | Diagnostics of LspDiagnostic list
    | LanguageStopped

type ILspTransport =
    abstract SendAsync: request: LspRequest * cancellationToken: CancellationToken -> Task<Result<LspResponse, string>>

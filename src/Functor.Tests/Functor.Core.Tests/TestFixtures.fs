namespace Functor.Core.Tests

open Functor.Domain.Diagnostics
open Functor.Domain.Editing
open Functor.Domain.Navigation

module TestFixtures =
    let applyEditing event model = EditingLogic.update event model

    let applyNavigation event model = NavigationLogic.update event model

    let position line column = { Line = line; Column = column }

    let diagnostic line message =
        { Severity = DiagnosticSeverity.Error
          Message = message
          RangeStart = position line 0
          RangeEnd = position line 1
          Code = None
          Source = None }

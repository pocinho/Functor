namespace Functor.Application

/// Application-facing configuration capability; persistence and serialization stay outside Application.
type ISettingsStore =
    abstract Load: unit -> Result<AppSettings, string>
    abstract Save: settings: AppSettings -> Result<unit, string>

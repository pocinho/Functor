namespace Functor.Application

type SettingsDraftState =
    { Form: SettingsFormModel option
      ThemeName: string
      SelectedThemeName: string option }

module SettingsDraftState =
    let empty =
        { Form = None
          ThemeName = ThemePreset.GraphiteDark
          SelectedThemeName = None }

    let fromAppSettings settings =
        { Form = Some(SettingsForm.fromAppSettings settings)
          ThemeName = settings.Theme.Preset
          SelectedThemeName = None }

    let selectNamedTheme name settings state =
        { Form = Some(SettingsForm.fromAppSettingsWithPreset settings.Theme.Preset settings)
          ThemeName = name
          SelectedThemeName = Some name }

    let selectCustom state =
        { state with
            Form = state.Form |> Option.map (fun form -> { form with ThemePreset = ThemePreset.Custom })
            SelectedThemeName = None }

    let applyPreset preset state =
        { state with
            Form = state.Form |> Option.map (SettingsForm.applyPreset preset)
            ThemeName = preset
            SelectedThemeName = None }

    let updateForm update state =
        match state.Form with
        | None -> state
        | Some current ->
            let updated = update current

            if updated = current then
                state
            else
                let shouldUseCustomPreset =
                    current.ThemePreset = ThemePreset.GraphiteDark
                    || current.ThemePreset = ThemePreset.GraphiteLight
                    || state.SelectedThemeName.IsSome

                if shouldUseCustomPreset then
                    { state with
                        Form = Some { updated with ThemePreset = ThemePreset.Custom }
                        SelectedThemeName = None }
                else
                    { state with Form = Some updated }

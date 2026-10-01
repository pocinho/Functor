namespace Functor.Application

type AppSettings =
    { Theme: ThemeSettings }

module AppSettings =
    let defaults =
        { Theme = ThemeSettings.defaultTheme }

    let fromTheme theme =
        { Theme = theme }
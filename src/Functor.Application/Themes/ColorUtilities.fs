namespace Functor.Application

open System.Globalization

module ColorUtilities =
    let toHex (color: uint32) =
        sprintf "#%s" (color.ToString("X8", CultureInfo.InvariantCulture))

namespace BibleAtlas.FSharp.Admission

open System

[<RequireQualifiedAccess>]
type LatitudeFailure = NonFiniteLatitude | LatitudeOutsideBounds

type Latitude = private Latitude of float

[<RequireQualifiedAccess>]
type LongitudeFailure = NonFiniteLongitude | LongitudeOutsideBounds

type Longitude = private Longitude of float

module Latitudes =
    let admit degrees =
        let southPole = -90.0
        let northPole = 90.0
        if not (Double.IsFinite degrees) then Error LatitudeFailure.NonFiniteLatitude
        elif degrees >= southPole && degrees <= northPole then Ok (Latitude degrees)
        else Error LatitudeFailure.LatitudeOutsideBounds
    let value (Latitude degrees) = degrees

module Longitudes =
    let admit degrees =
        let westernBound = -180.0
        let easternBound = 180.0
        if not (Double.IsFinite degrees) then Error LongitudeFailure.NonFiniteLongitude
        elif degrees >= westernBound && degrees <= easternBound then Ok (Longitude degrees)
        else Error LongitudeFailure.LongitudeOutsideBounds
    let value (Longitude degrees) = degrees

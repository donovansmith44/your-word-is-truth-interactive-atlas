module BibleAtlas.FSharp.Tests.Domain.CoordinatesLaws

open System
open FsCheck.Xunit
open BibleAtlas.FSharp.Admission

[<Property>]
let ``latitude rejects nonfinite and out of range values as distinct failures`` (degrees: float) =
    let expected =
        if Double.IsNaN degrees || Double.IsInfinity degrees then Error LatitudeFailure.NonFiniteLatitude
        elif degrees < -90.0 || degrees > 90.0 then Error LatitudeFailure.LatitudeOutsideBounds
        else Ok degrees
    (Latitudes.admit degrees |> Result.map Latitudes.value) = expected

[<Property>]
let ``longitude rejects nonfinite and out of range values as distinct failures`` (degrees: float) =
    let expected =
        if Double.IsNaN degrees || Double.IsInfinity degrees then Error LongitudeFailure.NonFiniteLongitude
        elif degrees < -180.0 || degrees > 180.0 then Error LongitudeFailure.LongitudeOutsideBounds
        else Ok degrees
    (Longitudes.admit degrees |> Result.map Longitudes.value) = expected

[<Property>]
let ``every nonfinite coordinate is refused including all NaN and infinity witnesses`` (sign: bool) =
    let infinite = if sign then Double.PositiveInfinity else Double.NegativeInfinity
    let latitude = [Double.NaN; infinite] |> List.map Latitudes.admit
    let longitude = [Double.NaN; infinite] |> List.map Longitudes.admit
    latitude = [Error LatitudeFailure.NonFiniteLatitude; Error LatitudeFailure.NonFiniteLatitude]
    && longitude = [Error LongitudeFailure.NonFiniteLongitude; Error LongitudeFailure.NonFiniteLongitude]


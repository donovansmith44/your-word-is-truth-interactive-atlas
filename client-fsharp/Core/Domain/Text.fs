namespace BibleAtlas.FSharp.Domain

open BibleAtlas.FSharp.Contract

type TextAnchor<'nodeId> =
    private { Start: int; End: int; Target: 'nodeId; Kind: EdgeKind }

type TextPart<'nodeId, 'partRole> =
    private { Role: 'partRole; Text: string; Anchors: TextAnchor<'nodeId> list }

type TextBody<'nodeId, 'partRole> = private TextBody of TextPart<'nodeId, 'partRole> list

type TextHeading<'nodeId> = private { Event: 'nodeId; Label: string; IsContinuation: bool }

type TextUnit<'nodeId, 'partRole> =
    private
        { Id: 'nodeId
          Reference: TextRef
          Label: string
          Body: TextBody<'nodeId, 'partRole>
          Headings: TextHeading<'nodeId> list
          Summary: Map<EdgeKind, int> }

[<RequireQualifiedAccess>]
type TextAnchorFailure = NegativeOffset | ReversedSpan | OutsideText

[<RequireQualifiedAccess>]
type TextPartFailure = AnchorOutsidePart

[<RequireQualifiedAccess>]
type TextUnitFailure = NotATextUnit | NegativeEdgeCount

module TextAnchors =
    let admit (text: string) (start: int) (finish: int) (target: 'nodeId) (kind: EdgeKind) : Result<TextAnchor<'nodeId>, TextAnchorFailure> =
        DomainSkeleton.pending "TextAnchors.admit"
    let target (anchor: TextAnchor<'nodeId>) : 'nodeId = anchor.Target
    let span (anchor: TextAnchor<'nodeId>) : Endpoints<int> = { First = anchor.Start; Last = anchor.End }
    let kind (anchor: TextAnchor<'nodeId>) : EdgeKind = anchor.Kind

module TextBodies =
    let part (role: 'partRole) (text: string) (anchors: TextAnchor<'nodeId> list) : Result<TextPart<'nodeId, 'partRole>, TextPartFailure> =
        DomainSkeleton.pending "TextBodies.part"
    let body (parts: TextPart<'nodeId, 'partRole> list) : TextBody<'nodeId, 'partRole> = TextBody parts
    let parts (TextBody parts) : TextPart<'nodeId, 'partRole> list = parts
    let text (part: TextPart<'nodeId, 'partRole>) : string = part.Text
    let role (part: TextPart<'nodeId, 'partRole>) : 'partRole = part.Role
    let anchors (part: TextPart<'nodeId, 'partRole>) : TextAnchor<'nodeId> list = part.Anchors

module TextUnits =
    let admit (id: 'nodeId) (kind: NodeKind) (reference: TextRef) (label: string) (body: TextBody<'nodeId, 'partRole>) (headings: TextHeading<'nodeId> list) (summary: Map<EdgeKind, int>) : Result<TextUnit<'nodeId, 'partRole>, TextUnitFailure> =
        DomainSkeleton.pending "TextUnits.admit"
    let heading (event: 'nodeId) (label: string) (isContinuation: bool) : TextHeading<'nodeId> =
        { Event = event; Label = label; IsContinuation = isContinuation }
    let id (unit: TextUnit<'nodeId, 'partRole>) : 'nodeId = unit.Id
    let reference (unit: TextUnit<'nodeId, 'partRole>) : TextRef = unit.Reference
    let label (unit: TextUnit<'nodeId, 'partRole>) : string = unit.Label
    let body (unit: TextUnit<'nodeId, 'partRole>) : TextBody<'nodeId, 'partRole> = unit.Body
    let headings (unit: TextUnit<'nodeId, 'partRole>) : TextHeading<'nodeId> list = unit.Headings
    let summary (unit: TextUnit<'nodeId, 'partRole>) : Map<EdgeKind, int> = unit.Summary

namespace BibleAtlas.FSharp.Domain

open BibleAtlas.FSharp.Contract

type Passage<'nodeId, 'mark> =
    private { Id: 'nodeId; Span: Endpoints<BibleRef>; Mark: 'mark; Label: string }

type BibleContainer<'nodeId, 'level> =
    private
        { Id: 'nodeId; Level: 'level; Label: string; Reference: BibleRef
          Children: 'nodeId list; Previous: 'nodeId option; Next: 'nodeId option }

type ConcordContainer<'nodeId, 'level> =
    private
        { Id: 'nodeId; Level: 'level; Label: string; Reference: ConcordRef
          Children: 'nodeId list; Previous: 'nodeId option; Next: 'nodeId option }

[<RequireQualifiedAccess>]
type Container<'nodeId, 'level> =
    | Bible of BibleContainer<'nodeId, 'level>
    | Concord of ConcordContainer<'nodeId, 'level>

type WholeRead<'nodeId> = private WholeRead of container: 'nodeId

type ContainerNavigation<'container, 'nodeId> =
    { Children: 'container -> 'nodeId list
      Previous: 'container -> 'nodeId option
      Next: 'container -> 'nodeId option
      WholeRead: 'container -> WholeRead<'nodeId> option }

type WholeChapter<'nodeId, 'partRole> =
    private { Container: 'nodeId; Units: NonEmpty<TextUnit<'nodeId, 'partRole>> }

[<RequireQualifiedAccess>]
type PassageFailure = OneUnit | ReversedUnits

[<RequireQualifiedAccess>]
type ContainerFailure = NotAContainer | WrongCorpus

[<RequireQualifiedAccess>]
type WholeChapterFailure = EmptyRead | IncompleteRead

module Passages =
    let admit (id: 'nodeId) (span: Endpoints<BibleRef>) (mark: 'mark) (label: string) (servedOrder: BibleRef -> BibleRef -> int) : Result<Passage<'nodeId, 'mark>, PassageFailure> =
        DomainSkeleton.pending "Passages.admit"
    let contains (servedOrder: BibleRef -> BibleRef -> int) (passage: Passage<'nodeId, 'mark>) (reference: BibleRef) : bool =
        DomainSkeleton.pending "Passages.contains"
    let id (passage: Passage<'nodeId, 'mark>) : 'nodeId = passage.Id
    let span (passage: Passage<'nodeId, 'mark>) : Endpoints<BibleRef> = passage.Span
    let mark (passage: Passage<'nodeId, 'mark>) : 'mark = passage.Mark
    let label (passage: Passage<'nodeId, 'mark>) : string = passage.Label

module BibleContainers =
    let admit (kind: NodeKind) (corpus: Corpus) (id: 'nodeId) (reference: BibleRef) (level: 'level) (label: string) (children: 'nodeId list) (previous: 'nodeId option) (next: 'nodeId option) : Result<BibleContainer<'nodeId, 'level>, ContainerFailure> =
        DomainSkeleton.pending "BibleContainers.admit"
    let navigation () : ContainerNavigation<BibleContainer<'nodeId, 'level>, 'nodeId> =
        DomainSkeleton.pending "BibleContainers.navigation"
    let id (container: BibleContainer<'nodeId, 'level>) : 'nodeId = container.Id
    let reference (container: BibleContainer<'nodeId, 'level>) : BibleRef = container.Reference
    let level (container: BibleContainer<'nodeId, 'level>) : 'level = container.Level
    let label (container: BibleContainer<'nodeId, 'level>) : string = container.Label

module ConcordContainers =
    let admit (kind: NodeKind) (corpus: Corpus) (id: 'nodeId) (reference: ConcordRef) (level: 'level) (label: string) (children: 'nodeId list) (previous: 'nodeId option) (next: 'nodeId option) : Result<ConcordContainer<'nodeId, 'level>, ContainerFailure> =
        DomainSkeleton.pending "ConcordContainers.admit"
    let navigation () : ContainerNavigation<ConcordContainer<'nodeId, 'level>, 'nodeId> =
        DomainSkeleton.pending "ConcordContainers.navigation"
    let id (container: ConcordContainer<'nodeId, 'level>) : 'nodeId = container.Id
    let reference (container: ConcordContainer<'nodeId, 'level>) : ConcordRef = container.Reference
    let level (container: ConcordContainer<'nodeId, 'level>) : 'level = container.Level
    let label (container: ConcordContainer<'nodeId, 'level>) : string = container.Label

module WholeChapters =
    let admit (container: 'nodeId) (units: TextUnit<'nodeId, 'partRole> list) (complete: bool) : Result<WholeChapter<'nodeId, 'partRole>, WholeChapterFailure> =
        DomainSkeleton.pending "WholeChapters.admit"
    let container (whole: WholeChapter<'nodeId, 'partRole>) : 'nodeId = whole.Container
    let units (whole: WholeChapter<'nodeId, 'partRole>) : NonEmpty<TextUnit<'nodeId, 'partRole>> = whole.Units

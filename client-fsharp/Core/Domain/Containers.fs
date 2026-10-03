namespace BibleAtlas.FSharp.Domain

open BibleAtlas.FSharp.Contract

type Passage<'nodeId, 'reference, 'mark> =
    private { Id: 'nodeId; Span: Endpoints<'reference>; Mark: 'mark; Label: string }

type BibleContainer<'nodeId, 'reference, 'level> =
    private
        { Id: 'nodeId; Level: 'level; Label: string; Reference: 'reference
          Children: 'nodeId list; Previous: 'nodeId option; Next: 'nodeId option }

type ConcordContainer<'nodeId, 'reference, 'level> =
    private
        { Id: 'nodeId; Level: 'level; Label: string; Reference: 'reference
          Children: 'nodeId list; Previous: 'nodeId option; Next: 'nodeId option }

[<RequireQualifiedAccess>]
type Container<'nodeId, 'reference, 'level> =
    | Bible of BibleContainer<'nodeId, 'reference, 'level>
    | Concord of ConcordContainer<'nodeId, 'reference, 'level>

type WholeRead<'nodeId> = private WholeRead of container: 'nodeId

type ContainerNavigation<'container, 'nodeId> =
    { Children: 'container -> 'nodeId list
      Previous: 'container -> 'nodeId option
      Next: 'container -> 'nodeId option
      WholeRead: 'container -> WholeRead<'nodeId> option }

type WholeChapter<'nodeId, 'reference, 'partRole> =
    private { Container: 'nodeId; Units: NonEmpty<TextUnit<'nodeId, 'reference, 'partRole>> }

[<RequireQualifiedAccess>]
type PassageFailure = OneUnit | ReversedUnits

[<RequireQualifiedAccess>]
type ContainerFailure = NotAContainer | WrongCorpus

[<RequireQualifiedAccess>]
type WholeChapterFailure = EmptyRead | IncompleteRead

module Passages =
    let admit (id: 'nodeId) (span: Endpoints<'reference>) (mark: 'mark) (label: string) (servedOrder: 'reference -> 'reference -> int) : Result<Passage<'nodeId, 'reference, 'mark>, PassageFailure> =
        DomainSkeleton.pending "Passages.admit"
    let contains (servedOrder: 'reference -> 'reference -> int) (passage: Passage<'nodeId, 'reference, 'mark>) (reference: 'reference) : bool =
        DomainSkeleton.pending "Passages.contains"
    let id (passage: Passage<'nodeId, 'reference, 'mark>) : 'nodeId = passage.Id
    let span (passage: Passage<'nodeId, 'reference, 'mark>) : Endpoints<'reference> = passage.Span
    let mark (passage: Passage<'nodeId, 'reference, 'mark>) : 'mark = passage.Mark
    let label (passage: Passage<'nodeId, 'reference, 'mark>) : string = passage.Label

module BibleContainers =
    let admit (kind: NodeKind) (corpus: Corpus) (id: 'nodeId) (reference: 'reference) (level: 'level) (label: string) (children: 'nodeId list) (previous: 'nodeId option) (next: 'nodeId option) : Result<BibleContainer<'nodeId, 'reference, 'level>, ContainerFailure> =
        DomainSkeleton.pending "BibleContainers.admit"
    let navigation () : ContainerNavigation<BibleContainer<'nodeId, 'reference, 'level>, 'nodeId> =
        DomainSkeleton.pending "BibleContainers.navigation"
    let id (container: BibleContainer<'nodeId, 'reference, 'level>) : 'nodeId = container.Id
    let reference (container: BibleContainer<'nodeId, 'reference, 'level>) : 'reference = container.Reference
    let level (container: BibleContainer<'nodeId, 'reference, 'level>) : 'level = container.Level
    let label (container: BibleContainer<'nodeId, 'reference, 'level>) : string = container.Label

module ConcordContainers =
    let admit (kind: NodeKind) (corpus: Corpus) (id: 'nodeId) (reference: 'reference) (level: 'level) (label: string) (children: 'nodeId list) (previous: 'nodeId option) (next: 'nodeId option) : Result<ConcordContainer<'nodeId, 'reference, 'level>, ContainerFailure> =
        DomainSkeleton.pending "ConcordContainers.admit"
    let navigation () : ContainerNavigation<ConcordContainer<'nodeId, 'reference, 'level>, 'nodeId> =
        DomainSkeleton.pending "ConcordContainers.navigation"
    let id (container: ConcordContainer<'nodeId, 'reference, 'level>) : 'nodeId = container.Id
    let reference (container: ConcordContainer<'nodeId, 'reference, 'level>) : 'reference = container.Reference
    let level (container: ConcordContainer<'nodeId, 'reference, 'level>) : 'level = container.Level
    let label (container: ConcordContainer<'nodeId, 'reference, 'level>) : string = container.Label

module WholeChapters =
    let admit (container: 'nodeId) (units: TextUnit<'nodeId, 'reference, 'partRole> list) (complete: bool) : Result<WholeChapter<'nodeId, 'reference, 'partRole>, WholeChapterFailure> =
        DomainSkeleton.pending "WholeChapters.admit"
    let container (whole: WholeChapter<'nodeId, 'reference, 'partRole>) : 'nodeId = whole.Container
    let units (whole: WholeChapter<'nodeId, 'reference, 'partRole>) : NonEmpty<TextUnit<'nodeId, 'reference, 'partRole>> = whole.Units

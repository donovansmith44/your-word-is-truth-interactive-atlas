namespace BibleAtlas.FSharp

type NonEmpty<'a> = private NonEmpty of head: 'a * tail: 'a list

module NonEmpty =
    let create head tail = NonEmpty(head, tail)
    let toList (NonEmpty(head, tail)) = head :: tail

Feature: the detail routes — pinned whole until the graph API subsumes them
  These six routes are consumed only by the atlas's own client today. Each
  is pinned as its entire response so that FOCUS can retire them one at a
  time with a visible diff, and so that no route is served without a promise.

  A whole-body pin is the honest shape for them and not a shortcut. A
  projection names the fields a consumer reads, and the consumer here is our
  own client, which reads all of them; restating each route's field list
  would be a second declaration of shapes `contracts/openapi.yaml` already
  owns, and the two would drift.

  Vocabulary:
    | projection | any of: books, catechism-item, catechism-list, chapter, contract, edge-page, eras, event, export-format, gazetteer, kretzmann-chapter, land-mask, landmarks, narrative-event, narratives, node-card, place, polities, sources, verse, version-root, vocabulary, xref-list |

  Scenario: the canon's books
    When I GET /api/books
    Then the consumed projection books equals fixture "books"

  Scenario: a chapter
    When I GET /api/chapter/GEN.1
    Then the consumed projection chapter equals fixture "chapter-gen-1"

  Scenario: a Kretzmann chapter
    When I GET /api/kretzmann/chapter/GEN.1
    Then the consumed projection kretzmann-chapter equals fixture "kretzmann-chapter-gen-1"

  Scenario: a catechism item
    When I GET /api/catechism/item/commandment-1
    Then the consumed projection catechism-item equals fixture "catechism-item-commandment-1"

  Scenario: an event's narrative positions
    When I GET /api/narrative/event/ab_ur
    Then the consumed projection narrative-event equals fixture "narrative-event-ab-ur"

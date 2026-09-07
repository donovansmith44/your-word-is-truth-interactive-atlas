namespace BibleAtlas.Client.Contracts;

// ============================================================================
// THE FRONTIER CONTRACT — spec §4e compiled (skeleton-first, owner-reviewed
// in-chat 2026-09-07: the change map, the matrix, and the enum-not-strings
// correction are all owner rulings).
//
// THE SMART-FRONTIER RULE (owner verbatim): "frontiers need to be smart.
// I.e., they display instances of types of nodes on the frontier iff both:
// 1) they exist for the focus node and 2) we have not opted out of
// displaying that node type on the frontier of the focus node."
//   display(section, focus) ⟺ exists(instances, focus)
//                           ∧ ¬optedOut(focus.Kind, sectionType)
// The FrontierMatrix below IS the opt-out registry (one line per cell —
// the one-line-knob acceptance bar). Existence is the runtime gate: no
// empty section ever renders (hidden-when-empty and hatch deliverability
// are instances of the same law). Both gates are conformance-tested, both
// directions (§4e: implementing-without-content and rendering-without-
// implementing each fail standing tests).
//
// Same discipline as every Contracts file: EXTEND-ONLY for implementers;
// member changes route through the controller.
// ============================================================================

/// <summary>THE focus-kind vocabulary (renamed from NodeKind per the
/// owner's reuse challenge — the GRAPH's NodeKind lives in the Rust types
/// crate (id.rs) and is a different vocabulary; graph-types/src/frontier.rs
/// is the cross-language authority this file mirrors, incl. the
/// FocusKind→graph-NodeKind bridge). THE vocabulary — one enum (owner: "use our node type
/// enums rather than strings"). The compiler now catches a typo'd kind in
/// any matrix cell; a misspelled string would have silently opted a kind
/// out.</summary>
public enum FocusKind
{
    Verse,
    Passage,
    Chapter,
    Book,
    Author,
    Place,
    Person,
    Event,
    Catechism,
    Year,
    TimeAndPlace,
    PolityDelta,
    CommentaryItem,
    ConcordUnit,
}

/// <summary>The ONE string↔enum mapping site. IExplorable.Kind is
/// stringly-typed today; every kind-string switch in the codebase migrates
/// through here over time. An unknown string throws loud (fail-loud law) —
/// never a guess, never a default.</summary>
public static class FocusKinds
{
    public static FocusKind Parse(string kind) => kind switch
    {
        "Verse" => FocusKind.Verse,
        "Passage" => FocusKind.Passage,
        "Chapter" => FocusKind.Chapter,
        "Book" => FocusKind.Book,
        "Author" => FocusKind.Author,
        "Place" => FocusKind.Place,
        "Person" => FocusKind.Person,
        "Event" => FocusKind.Event,
        "Catechism" => FocusKind.Catechism,
        "Year" => FocusKind.Year,
        "TimeAndPlace" => FocusKind.TimeAndPlace,
        "PolityDelta" => FocusKind.PolityDelta,
        "CommentaryItem" => FocusKind.CommentaryItem,
        "ConcordUnit" => FocusKind.ConcordUnit,
        _ => throw new NotSupportedException(
            $"Unknown node kind '{kind}' — a new IExplorable kind must be " +
            "added to FocusKind/FocusKinds.Parse (Contracts, controller-routed) " +
            "before it can traverse the frontier contract."),
    };
}

// ── CAPABILITY INTERFACES (§4e) ─────────────────────────────────────────
// One per non-core section family. A kind implements one IFF it can
// genuinely provide the content today — the presence IS the claim; an
// event acquires cross-references the day event-level xref data exists,
// by joining the matrix cell, never by an empty section.
public interface IHasCrossReferences { }
public interface IHasParallels { }
public interface IHasEventMembership { }
public interface IHasPassageMembership { }
public interface IHasPersons { }
public interface IHasCatechismSupport { }
/// <summary>EVT-3: the refs-list under prior/following (accounts as refs,
/// never verse contents, per the owner's chronological-traversal order).</summary>
public interface IHasChronology { }
/// <summary>EVT-3: explorable Time:/Place: header rows; Place exploration =
/// f(location, event time) with the map-open side effect (§5 hatch).</summary>
public interface IHasTimeAndPlace { }
/// <summary>EVT-3: "events are keys that map to sets of Biblical accounts"
/// (owner verbatim) — the accounts render as explorable passages.</summary>
public interface IHasAccounts { }

// ── THE OPT-OUT REGISTRY (the matrix as compiled data) ──────────────────
// Absence of a kind from a set = opted out for that kind. Editing a cell
// is a ONE-LINE change. Body-only sections (Place's four, Catechism's
// four, PolityDelta's three, Person, CommentaryItem, the Chapter card)
// are core focus-presentation, not matrix entries.
public static class FrontierMatrix
{
    public static readonly IReadOnlySet<FocusKind> CrossReferences =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };
    // Event deliberately absent from CrossReferences — the owner's
    // calibration row ("we are not yet at the point of being able ot
    // provide lots of cross references for events").

    public static readonly IReadOnlySet<FocusKind> Parallels =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };

    public static readonly IReadOnlySet<FocusKind> EventMembership =
        new HashSet<FocusKind> { FocusKind.Verse };

    public static readonly IReadOnlySet<FocusKind> PassageMembership =
        new HashSet<FocusKind> { FocusKind.Verse };

    public static readonly IReadOnlySet<FocusKind> Persons =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };

    public static readonly IReadOnlySet<FocusKind> CatechismSupport =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };

    public static readonly IReadOnlySet<FocusKind> Chronology =
        new HashSet<FocusKind> { FocusKind.Event };

    public static readonly IReadOnlySet<FocusKind> TimeAndPlace =
        new HashSet<FocusKind> { FocusKind.Event };

    public static readonly IReadOnlySet<FocusKind> Accounts =
        new HashSet<FocusKind> { FocusKind.Event };
}

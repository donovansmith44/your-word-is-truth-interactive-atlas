namespace BibleAtlas.Client.Contracts;

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

public interface IHasCrossReferences { }
public interface IHasParallels { }
public interface IHasEventMembership { }
public interface IHasPassageMembership { }
public interface IHasPersons { }
public interface IHasCatechismSupport { }
public interface IHasChronology { }
public interface IHasTimeAndPlace { }
public interface IHasAccounts { }

public static class FrontierMatrix
{
    public static readonly IReadOnlySet<FocusKind> CrossReferences =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };
    // Event is deliberately excluded: cross-reference data isn't available for events yet.

    public static readonly IReadOnlySet<FocusKind> Parallels =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };

    public static readonly IReadOnlySet<FocusKind> EventMembership =
        new HashSet<FocusKind> { FocusKind.Verse };

    public static readonly IReadOnlySet<FocusKind> PassageMembership =
        new HashSet<FocusKind> { FocusKind.Verse };

    public static readonly IReadOnlySet<FocusKind> Persons =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };

    // Deliberately narrower than the Rust side's ConcordUnit claim: this set is read by client
    // conformance tests as "where CatechismSeamSection renders" (the verse end only) -- the
    // paragraph end renders through its own AppliesTo, not through this set.
    public static readonly IReadOnlySet<FocusKind> CatechismSupport =
        new HashSet<FocusKind> { FocusKind.Verse, FocusKind.Passage };

    public static readonly IReadOnlySet<FocusKind> Chronology =
        new HashSet<FocusKind> { FocusKind.Event };

    public static readonly IReadOnlySet<FocusKind> TimeAndPlace =
        new HashSet<FocusKind> { FocusKind.Event };

    public static readonly IReadOnlySet<FocusKind> Accounts =
        new HashSet<FocusKind> { FocusKind.Event };
}

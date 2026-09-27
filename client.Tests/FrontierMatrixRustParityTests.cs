using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.Tests;

/// <summary>
/// MID-BATCH CONTROLLER COMMIT (bc4989c, owner-ordered, "write it in rust
/// and make sure it compiles before implementing"; renamed 0fc6f7f
/// NodeKind→FocusKind on both sides): <c>graph-types/src/frontier.rs</c>
/// is now the CROSS-LANGUAGE AUTHORITY -- the C# <see cref="FrontierMatrix"/>
/// (Contracts/Frontier.cs) is a MIRROR of it, not an independent
/// declaration. This is the parity test the controller asked for (the AQC
/// scenario-count parity precedent, applied to the frontier matrix): it
/// holds the C# mirror to the Rust truth.
///
/// FORM CHOSEN (disclosed, per the controller's own menu of options): this
/// batch's own house rule (<c>GraphExplorableClientTests.cs</c>'s doc
/// comment) is that C# unit tests never spin up a live server or shell out
/// to `cargo` -- so this is NOT a generated fixture and NOT a runtime FFI
/// call into the Rust crate (either would add real coupling: a build-time
/// codegen step, or a cross-process dependency this test suite has never
/// needed before). Instead: <see cref="RustAllows"/> below is a byte-for-
/// byte TRANSCRIPTION of every match arm in <c>graph-types/src/frontier.rs</c>'s
/// own <c>allows(FocusKind, Capability)</c> (as of commit bc4989c/0fc6f7f)
/// -- a literal, committed expected-table, exactly the form the controller
/// named ("derive both sides in-test from their own declarations and
/// compare cell-by-cell against a shared literal expected-table..., updated
/// only with both"). ZERO runtime coupling: this file imports nothing from
/// the Rust crate and starts no process. The cost is honest, not hidden --
/// this transcription can drift from the real Rust source the same way any
/// hand-copied literal can; <see cref="ExhaustivenessGuard"/> below at
/// least guarantees the copy itself never silently loses a cell.
///
/// MAINTENANCE (stated once, binding on every future editor): a cell
/// changing in EITHER file (Contracts/Frontier.cs's <see cref="FrontierMatrix"/>
/// or graph-types/src/frontier.rs's <c>allows</c>) must change here too, in
/// the SAME commit -- this test is what makes a one-sided edit fail loud
/// instead of silently diverging.
/// </summary>
public class FrontierMatrixRustParityTests
{
    // The 9 capability families, named exactly as graph-types/src/frontier.rs's
    // own `Capability` enum -- and as Contracts/Frontier.cs's own
    // `FrontierMatrix` field names (case mirrors the Rust variant, e.g.
    // "CrossReferences"), so ByCapabilityName below reads either side with
    // the identical string.
    private static readonly string[] Capabilities =
    {
        "CrossReferences", "Parallels", "EventMembership", "PassageMembership",
        "Persons", "CatechismSupport", "Chronology", "TimeAndPlace", "Accounts",
    };

    /// The C# side's own real declaration -- reads FrontierMatrix's actual
    /// field for the named capability (reflection-free: an explicit switch,
    /// so a renamed/removed field fails this test to COMPILE, not silently
    /// resolves to nothing).
    private static IReadOnlySet<FocusKind> CSharpMatrixSet(string capability) => capability switch
    {
        "CrossReferences" => FrontierMatrix.CrossReferences,
        "Parallels" => FrontierMatrix.Parallels,
        "EventMembership" => FrontierMatrix.EventMembership,
        "PassageMembership" => FrontierMatrix.PassageMembership,
        "Persons" => FrontierMatrix.Persons,
        "CatechismSupport" => FrontierMatrix.CatechismSupport,
        "Chronology" => FrontierMatrix.Chronology,
        "TimeAndPlace" => FrontierMatrix.TimeAndPlace,
        "Accounts" => FrontierMatrix.Accounts,
        _ => throw new NotSupportedException($"Unknown capability '{capability}' -- add it to BOTH sides."),
    };

    /// The Rust side's own transcription -- see this class's own doc
    /// comment for the "why a literal port, not codegen/FFI" story. Every
    /// arm below corresponds 1:1 to a match arm in
    /// graph-types/src/frontier.rs's own `allows` (bc4989c/0fc6f7f).
    private static bool RustAllows(FocusKind kind, string capability) => (kind, capability) switch
    {
        (FocusKind.Verse, "CrossReferences" or "Parallels" or "EventMembership"
            or "PassageMembership" or "Persons" or "CatechismSupport") => true,
        (FocusKind.Verse, "Chronology" or "TimeAndPlace" or "Accounts") => false,

        (FocusKind.Passage, "CrossReferences" or "Parallels" or "Persons"
            or "CatechismSupport") => true,
        (FocusKind.Passage, "EventMembership" or "PassageMembership"
            or "Chronology" or "TimeAndPlace" or "Accounts") => false,

        (FocusKind.Event, "Chronology" or "TimeAndPlace" or "Accounts") => true,
        (FocusKind.Event, "CrossReferences" or "Parallels" or "EventMembership"
            or "PassageMembership" or "Persons" or "CatechismSupport") => false,

        // Body-only kinds: every capability opted out, both sides.
        (FocusKind.Chapter or FocusKind.Book or FocusKind.Author or FocusKind.Place
            or FocusKind.Person or FocusKind.Catechism or FocusKind.Year
            or FocusKind.TimeAndPlace or FocusKind.PolityDelta
            or FocusKind.CommentaryItem or FocusKind.ConcordUnit, _) => false,

        _ => throw new NotSupportedException($"RustAllows: no transcribed arm for ({kind}, {capability}) -- the Rust match is exhaustive; this port must be too."),
    };

    [Fact]
    public void EveryCellAgreesWithTheRustTranscription()
    {
        var kinds = Enum.GetValues<FocusKind>();
        var checkedCells = 0;
        foreach (var kind in kinds)
        {
            foreach (var capability in Capabilities)
            {
                var cSharp = CSharpMatrixSet(capability).Contains(kind);
                var rust = RustAllows(kind, capability);
                Assert.True(cSharp == rust,
                    $"Parity mismatch at ({kind}, {capability}): C# FrontierMatrix says {cSharp}, graph-types/src/frontier.rs says {rust}.");
                checkedCells++;
            }
        }

        // Never-vacuous guard (ExhaustivenessGuard's own name): 14 kinds x
        // 9 capabilities = 126 cells. If FocusKind or the Capabilities list
        // above ever drifts out of sync with either real declaration, this
        // pins the exact count so the drift is loud, not silent.
        Assert.Equal(14 * 9, checkedCells);
    }

    /// The owner's own calibration row, pinned on BOTH sides at once: an
    /// Event frontier never implements cross-references (until event-level
    /// xref data exists) -- the SAME fact graph-types/src/frontier.rs's own
    /// `event_has_no_cross_references` test pins on the Rust side.
    [Fact]
    public void CalibrationRow_EventHasNoCrossReferences_OnBothSides()
    {
        Assert.False(FrontierMatrix.CrossReferences.Contains(FocusKind.Event));
        Assert.False(RustAllows(FocusKind.Event, "CrossReferences"));
    }

    /// EVT-3's own three Event capabilities, pinned on BOTH sides -- the
    /// SAME fact graph-types/src/frontier.rs's own
    /// `event_implements_chronology_time_and_place_accounts` test pins.
    [Fact]
    public void EventImplementsChronologyTimeAndPlaceAccounts_OnBothSides()
    {
        foreach (var capability in new[] { "Chronology", "TimeAndPlace", "Accounts" })
        {
            Assert.True(CSharpMatrixSet(capability).Contains(FocusKind.Event));
            Assert.True(RustAllows(FocusKind.Event, capability));
        }
    }
}

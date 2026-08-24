namespace BibleAtlas.Client;

/// <summary>
/// TRAV-1 (controller decision 5, CHAP-HOVER-1): the first interaction-
/// grammar constant this house names explicitly -- one shared dwell delay
/// for every "a bare/brief hover does NOTHING; a sustained hover reveals a
/// TRANSIENT peek; pointer-leave dismisses it; click still commits" surface
/// in this app. Today's two consumers: the Narrative/Chronology arrow peek
/// (<see cref="Components.ArrowNav"/>) and the chapter-head peek
/// (<c>Pages/Reader.razor</c>) -- a future dwell surface should adopt this
/// constant rather than inventing its own.
///
/// ~350-400ms, deliberately insensitive -- the owner's own words, verbatim,
/// for the arrow peek ("not super sensitive, so some delay so that you're
/// not accidentally getting hover boxes all the time"), and exactly the
/// defect CHAP-HOVER-1 fixes for the chapter head the other direction
/// ("chapter headers... give a hover box that i have to x out of if i so
/// much as tickle the chapter button"). Long enough that an ordinary mouse
/// pass-over (moving toward a click, or toward something else entirely)
/// essentially never trips it; short enough that a genuine pause to look
/// reveals the peek promptly, not sluggishly.
///
/// Each consumer owns its own dwell TIMER (a `CancellationTokenSource` +
/// `Task.Delay`, the SAME shape Reader.razor's own pre-existing xref-hover
/// grace-period close (`ScheduleHoverClose`/`DelayedHoverClose`) already
/// established for the dismiss side) -- this file supplies only the one
/// number every one of them must agree on, not a shared component every
/// dwell surface is forced to render through (their own peek CONTENT
/// differs -- verse text for arrows, chapter metadata for the chapter
/// head -- only the TIMING is common).
/// </summary>
public static class DwellTiming
{
    public const int PeekDelayMs = 375;
}

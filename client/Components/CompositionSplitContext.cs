using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Components;

/// <summary>
/// Batch VC-1 fix round 1 (Adjudication F): what <see cref="CompositionSplit"/>
/// hands its own <c>ChildContent</c> template every render -- the SAME
/// role/callback data every host-glue duplication site used to hand-derive
/// independently (Adjudication F's own inventory rows #1/#2/#11/#12), now
/// computed exactly ONCE, by the one component that owns the atom read.
/// </summary>
/// <param name="IsSplitOpen">Mirrors <see cref="CompositionSplit.IsSplitOpen"/>
/// -- true whenever this HostName is part of a live split-h pairing, in
/// EITHER role.</param>
/// <param name="IsHost">Mirrors <see cref="CompositionSplit.IsHost"/> --
/// meaningful only while <paramref name="IsSplitOpen"/>.</param>
/// <param name="InvokeHatch">Invokes this HostName's own declared
/// enter-split hatch (looked up by name through the registry) -- bind a
/// page's own "Open the map beside the text"/"Read beside the reader"
/// button directly to this, no page-local lookup method needed anymore.</param>
/// <param name="RequestClose">"Close ME" -- meaningful only while
/// <paramref name="IsSplitOpen"/> and NOT <paramref name="IsHost"/> (a
/// guest's own close button); dispatches <c>CloseGuest</c>. A host's own
/// self-close (e.g. Reader's "close the reader, keep the map," a real
/// navigation) is NOT this -- it stays each host's own bespoke method,
/// wired directly, since it is genuinely per-host behavior.</param>
public sealed record CompositionSplitContext(bool IsSplitOpen, bool IsHost, EventCallback InvokeHatch, EventCallback RequestClose);

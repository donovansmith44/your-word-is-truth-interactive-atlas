using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;

namespace BibleAtlas.Client.Tests;

// Batch UX-1 (FRONTIER-ORDER-1, owner order verbatim: "when clicking on a
// verse, things should be ordered as such (visually): Verse, Event,
// Catechism, Parallels, then cross references"): direct, isolated proof
// that PopoverSectionRegistry.Providers -- the ORDER-KEYED registry
// (Explore/PopoverSections.cs) ExplorerPopover.razor renders straight
// through, unchanged -- actually resolves the VERSE/PASSAGE frontier in
// the owner's own ruled sequence. Pinned against the REAL registry output
// (not a copy of the Order values) so a future edit that reorders a
// registration line without meaning to breaks this test immediately,
// naming exactly which two providers swapped.
public class PopoverSectionRegistryTests
{
    private static int IndexOfProvider<T>() where T : IPopoverSectionProvider =>
        PopoverSectionRegistry.Providers.ToList().FindIndex(p => p is T);

    [Fact]
    public void A_passages_text_comes_before_its_catechism_and_its_catechism_before_its_cross_references()
    {
        Assert.True(IndexOfProvider<PassageTextSection>() < IndexOfProvider<PassageCatechismSection>());
        Assert.True(IndexOfProvider<PassageCatechismSection>() < IndexOfProvider<PassageCrossRefsSection>());
    }

    [Fact]
    public void Every_passage_provider_appears_exactly_once()
    {
        var providers = PopoverSectionRegistry.Providers;
        Assert.Single(providers, p => p is PassageTextSection);
        Assert.Single(providers, p => p is PassageCatechismSection);
        Assert.Single(providers, p => p is PassageCrossRefsSection);
    }

    // Batch ATTEST-1, THE OWNER'S PLACEMENT AMENDMENT (verbatim): "let's
    // have a 'Similar Accounts' or something similar added to the frontier
    // part of the UI where it was getting pulled in as a parallel account.
    // Have that section be right below the 'Parallel ..' section."
    //
    // ADJACENCY, not a number. Asserting `Order == 161` would pass while a
    // future provider registered at 160.5-equivalent (any value between the
    // two, or a tie resolved by array position) silently slid in between
    // and pushed Similar Accounts away from the parallels it is supposed to
    // sit under. What the owner asked for is that a row which was WRONGLY
    // appearing as a parallel account moves down EXACTLY ONE section -- so
    // that is what is pinned: consecutive indices in the real resolved
    // registry, nothing permitted between them.
    [Fact]
    public void SimilarAccountsRendersImmediatelyBelowParallelAccounts()
    {
        var parallels = IndexOfProvider<EventWitnessesSection>();
        var similar = IndexOfProvider<EventAnaloguesSection>();
        Assert.True(parallels >= 0, "PARALLEL ACCOUNTS (EventWitnessesSection) must be registered");
        Assert.True(similar >= 0, "SIMILAR ACCOUNTS (EventAnaloguesSection) must be registered");
        Assert.Equal(parallels + 1, similar);
    }

    // ... and MENTIONED IN follows both, never between them (it is a
    // different claim again -- scriptural basis, not a relation to another
    // event -- and the owner's amendment named the analogue specifically as
    // the section that sits under the parallels).
    [Fact]
    public void MentionedInFollowsSimilarAccounts()
    {
        Assert.True(IndexOfProvider<EventAnaloguesSection>() < IndexOfProvider<EventMentionsSection>());
    }

    [Fact]
    public void EveryEventFrontierProviderAddedThisBatchAppearsExactlyOnce()
    {
        var providers = PopoverSectionRegistry.Providers;
        Assert.Single(providers, p => p is EventAnaloguesSection);
        Assert.Single(providers, p => p is EventMentionsSection);
    }

    // Batch PROV-1 (owner order 2, "add a ? button on our frontier
    // interface"): the EVENT's own "?" sits DIRECTLY under the event
    // header, one line above the time/place block EVT-META-TOP-1 put
    // there ("time + place block should be moved to the top, right below
    // the event header"). Asserted as ADJACENCY over the real resolved
    // registry, the same discipline SimilarAccountsRendersImmediatelyBelow
    // ParallelAccounts above uses -- so a future provider registered at
    // any Order value between 134 and 135 fails loud rather than quietly
    // pushing the attribution away from the title it attributes.
    [Fact]
    public void EventProvenanceRendersImmediatelyAboveTheEventsTimeAndPlaceBlock()
    {
        var provenance = IndexOfProvider<EventProvenanceSection>();
        var timePlace = IndexOfProvider<EventDateAndPlacesSection>();
        Assert.True(provenance >= 0, "the EVENT's own provenance affordance (EventProvenanceSection) must be registered");
        Assert.True(timePlace >= 0, "EventDateAndPlacesSection must be registered");
        Assert.Equal(provenance + 1, timePlace);
    }

    [Fact]
    public void TheEventProvenanceProviderIsRegisteredExactlyOnce()
    {
        Assert.Single(PopoverSectionRegistry.Providers, p => p is EventProvenanceSection);
    }

    /// <summary>
    /// D3 (owner, 2026-09-15): the catechism ↔ Book of Concord traversal is
    /// symmetric -- the item end ("IN THE BOOK OF CONCORD", right after THE
    /// SCRIPTURES) and the paragraph end ("THE SMALL CATECHISM" on a
    /// ConcordUnit) are each registered exactly once.
    /// </summary>
    [Fact]
    public void CatechismItemAndConcordUnitHaveSymmetricCatechismLinkSections()
    {
        var providers = PopoverSectionRegistry.Providers.ToList();
        Assert.Equal(1, providers.Count(p => p is CatechismInConcordSection));
        Assert.True(IndexOfProvider<CatechismScripturesSection>() < IndexOfProvider<CatechismInConcordSection>(), "the Book of Concord list follows THE SCRIPTURES on an item's card");
        Assert.True(new CatechismInConcordSection().AppliesTo(new CatechismNode("first-commandment", "The First Commandment")));
    }

    /// <summary>
    /// D5 (owner, 2026-09-15): a person's card reads Life, Events, Family,
    /// then the verse list -- each provider registered once, in that order.
    /// </summary>
    [Fact]
    public void PersonCardReadsLifeEventsFamilyThenMentions()
    {
        var providers = PopoverSectionRegistry.Providers.ToList();
        foreach (var t in new[] { typeof(PersonLifeSection), typeof(PersonEventsSection), typeof(PersonFamilySection), typeof(PersonCardAndMentionsSection) })
        {
            Assert.Equal(1, providers.Count(p => p.GetType() == t));
        }
        Assert.True(IndexOfProvider<PersonLifeSection>() < IndexOfProvider<PersonEventsSection>());
        Assert.True(IndexOfProvider<PersonEventsSection>() < IndexOfProvider<PersonFamilySection>());
        Assert.True(IndexOfProvider<PersonFamilySection>() < IndexOfProvider<PersonCardAndMentionsSection>());
        var person = new PersonNode("abraham_1", "Abraham");
        Assert.True(new PersonLifeSection().AppliesTo(person) && new PersonFamilySection().AppliesTo(person));
        Assert.False(new PersonLifeSection().AppliesTo(new CatechismNode("commandment-1", "The First Commandment")));
    }
}

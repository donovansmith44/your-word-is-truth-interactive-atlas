using System.Diagnostics;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public interface IPresenter
{
    Task<Presentation?> Present(PresentationRequest request);
}

public sealed class GraphPresenter : IPresenter
{
    private const string ProvenanceField = "Provenance";
    private const string WindowField = "Window";
    private const string ReignField = "Reign";
    private const string CanonicalNameField = "Canonical name";
    private const string EstablishedField = "Established";
    private const string DestroyedField = "Destroyed";

    public async Task<Presentation?> Present(PresentationRequest request) =>
        Presentation.Of(request.Element.Kind, request.Surface) is { } form ? await PresentAs(request.Element, form) : null;

    private static async Task<Presentation> PresentAs(Explorable element, Presentation.Form form) => form switch
    {
        Presentation.Form.Card or Presentation.Form.Sequence or Presentation.Form.Text => CardOf(element),
        Presentation.Form.Geography => await element.Kind.Match(
            node: kind => GeographyOf(element, kind, element.Record ?? throw new UnreachableException($"{element.Id} resolved as a node without its record")),
            edge: kind => throw new UnreachableException($"{kind} is an edge kind, and no edge has a geography")),
    };

    private static Presentation.Card CardOf(Explorable element) =>
        new(
            element.Label,
            new[]
            {
                Field(WindowField, element.Record?.Map?.Window.Label),
                Field(WindowField, element.Record?.Era?.Window.Label),
                Field(CanonicalNameField, element.Record?.Place?.CanonicalName),
                Field(EstablishedField, element.Record?.Place?.Established?.Label),
                Field(DestroyedField, element.Record?.Place?.Destroyed?.Label),
                Field(ReignField, element.Record?.Polity?.Reign.Label),
                Field(ProvenanceField, element.Provenance),
            }.OfType<Presentation.Field>().ToList());

    private static Presentation.Field? Field(string name, string? value) => value is null ? null : new Presentation.Field(name, value);

    private static async Task<Presentation> GeographyOf(Explorable element, NodeKind kind, NodeRecord record) => kind switch
    {
        NodeKind.Map => await Bounded(element, Served(record.Map, record).Window),
        NodeKind.Era => await Bounded(element, Served(record.Era, record).Window),
        NodeKind.Place => new Presentation.Geography(new Frame.Current(), new Emphasis.Site(RefOf(record), Served(record.Place, record).Lat, Served(record.Place, record).Lon)),
        NodeKind.Polity => new Presentation.Geography(new Frame.Current(), new Emphasis.Territory(RefOf(record), Served(record.Polity, record).Reign)),
        NodeKind.TextUnit or NodeKind.Container or NodeKind.Event or NodeKind.Narrative or NodeKind.Person
            or NodeKind.Anchor or NodeKind.CatechismItem or NodeKind.Source or NodeKind.Translation
            or NodeKind.PeopleGroup or NodeKind.CommentaryItem or NodeKind.LexiconEntry =>
            throw new UnreachableException($"{kind} has no geography"),
    };

    private static async Task<Presentation> Bounded(Explorable element, TimeRange window) =>
        new Presentation.Geography(
            new Frame.Bounded(window, await Paging.FirstLink(element, EdgeKind.PrecedesIn), await Paging.FirstLink(element, EdgeKind.FollowsIn)),
            new Emphasis.None());

    private static T Served<T>(T? detail, NodeRecord record)
        where T : class =>
        detail ?? throw new ContractBreach($"{record.Id} is a {record.Kind} served without its {typeof(T).Name}");

    private static NodeRef RefOf(NodeRecord record) => new(id: record.Id, kind: record.Kind, label: record.Label);
}

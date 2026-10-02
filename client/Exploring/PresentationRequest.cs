namespace BibleAtlas.Client.Exploring;

public sealed record PresentationRequest(Explorable Element, Surface Surface)
{
    public bool Offers(Link link) => Presentation.Offers(link, Surface);

    public bool Equals(PresentationRequest? other) =>
        other is not null && Explorable.Served.Equals(Element, other.Element) && Surface == other.Surface;

    public override int GetHashCode() => HashCode.Combine(Explorable.Served.GetHashCode(Element), Surface);
}

namespace BibleAtlas.Client.Explore;

public sealed record PresentationRequest(Explorable Element, Surface Surface)
{
    public bool Offers(Link link) => Presentation.Offers(link, Surface);
}

namespace BibleAtlas.Client.Contracts;

public interface IView
{
    string Name { get; }
    IReadOnlyList<IViewComponent> Components { get; }
    IReadOnlyList<IEscapeHatch> EscapeHatches { get; }
}

public interface IViewComponent { }

public interface IEscapeHatch
{
    string Kind { get; }
    Task Invoke();
}

public interface IViewComposition : IView
{
    IReadOnlyList<IView> Members { get; }
    ICompositionLayout Layout { get; }
}

public interface ICompositionLayout
{
    string Kind { get; }
}

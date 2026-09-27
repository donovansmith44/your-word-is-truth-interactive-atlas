namespace BibleAtlas.Client;

public static class FeatureFlags
{
    // static readonly, not const: a const false made the gated render branch provably
    // unreachable and tripped CS0162 on an otherwise warning-free build.
    public static readonly bool XrefSuperscripts = true;
}

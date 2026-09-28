using NJsonSchema;
using NJsonSchema.CodeGeneration;

namespace BibleAtlas.Client.ContractGenerator;

public sealed class PascalCasePropertyNames : IPropertyNameGenerator
{
    private static readonly char[] WordSeparators = ['_', '-'];

    public string Generate(JsonSchemaProperty property) =>
        string.Concat(property.Name.Split(WordSeparators, StringSplitOptions.RemoveEmptyEntries).Select(Capitalised));

    private static string Capitalised(string word) => char.ToUpperInvariant(word[0]) + word[1..];
}

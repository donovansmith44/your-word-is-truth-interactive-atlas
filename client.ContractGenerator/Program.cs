using BibleAtlas.Client.ContractGenerator;
using NSwag;
using NSwag.CodeGeneration.CSharp;

var (contract, output) = (args[0], args[1]);

var document = await OpenApiYamlDocument.FromFileAsync(contract);
ContractGeneration.CloseDiscriminatedUnions(document);

Directory.CreateDirectory(Path.GetDirectoryName(output)!);
var settings = ContractGeneration.Settings(document);
var resolver = IdentityTypeResolver.For(document, settings.CSharpGeneratorSettings);
await File.WriteAllTextAsync(output, new CSharpClientGenerator(document, settings, resolver).GenerateFile() + IdentityTypes.Emit(document));

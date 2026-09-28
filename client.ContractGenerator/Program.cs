using BibleAtlas.Client.ContractGenerator;
using NSwag;
using NSwag.CodeGeneration.CSharp;

var (contract, output) = (args[0], args[1]);

var document = await OpenApiYamlDocument.FromFileAsync(contract);

Directory.CreateDirectory(Path.GetDirectoryName(output)!);
await File.WriteAllTextAsync(output, new CSharpClientGenerator(document, ContractGeneration.Settings(document)).GenerateFile());

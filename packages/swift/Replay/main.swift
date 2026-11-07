import Foundation
import GSX

struct Case: Decodable {
    struct Item: Decodable {
        let id: String
        let cover: String?
    }

    struct Given: Decodable {
        let media: MediaKind?
        let mediaCount: Int?
        let placement: Placement?
        let title: String?
        let targetUrl: String?
        let maxCharacters: UInt32?
        let charactersReservedPerUrl: UInt32?
        let captionLimit: UInt32?
        let descriptionLimit: UInt32?
        let entitiesAllowed: Bool?
        let items: [MediaFact]?
    }

    let markdown: String
    let media: [Item]
    let target: String?
    let facts: Given?
}

guard GSX.targets.contains(where: { $0.name == "x-plain" }) else { fatalError("targets: \(GSX.targets)") }

let arguments = CommandLine.arguments.dropFirst()
guard let out = arguments.first, arguments.count > 1 else {
    FileHandle.standardError.write(Data("usage: App <out.jsonl> <corpus.jsonl>...\n".utf8))
    exit(2)
}

var lines: [String] = []
let decoder = JSONDecoder()
for file in arguments.dropFirst() {
    let text = try String(contentsOfFile: file, encoding: .utf8)
    for line in text.split(separator: "\n").dropFirst() {
        guard let input = try? decoder.decode(Case.self, from: Data(line.utf8)) else {
            lines.append("-")
            if !line.contains(#""target":"#) { lines.append("-") }
            continue
        }
        let media = input.media.map { Media(id: $0.id, cover: $0.cover) }
        let given = input.facts
        let facts = Facts(
            media: given?.media,
            mediaCount: given?.mediaCount ?? 0,
            placement: given?.placement,
            title: given?.title,
            targetUrl: given?.targetUrl,
            maxCharacters: given?.maxCharacters,
            charactersReservedPerUrl: given?.charactersReservedPerUrl,
            captionLimit: given?.captionLimit,
            descriptionLimit: given?.descriptionLimit,
            entitiesAllowed: given?.entitiesAllowed ?? false,
            items: given?.items
        )
        let json = try input.target.map { try GSX.projectJSON(input.markdown, target: $0, facts: facts, media: media) }
            ?? GSX.parseJSON(input.markdown, media: media)
        do {
            if let target = input.target {
                _ = try GSX.project(input.markdown, target: target, facts: facts, media: media)
                guard json.hasPrefix(#"{"projection":"#) else { fatalError("project returned a projection for \(json)") }
            } else {
                _ = try GSX.parse(input.markdown, media: media)
                guard json.hasPrefix(#"{"document":"#) else { fatalError("parse returned a document for \(json)") }
            }
        } catch let error as ParseError {
            guard json.hasPrefix(#"{"error":{"code":"\#(error.code.rawValue)""#) else { fatalError("\(error) for \(json)") }
        }
        lines.append(json)
        if input.target == nil {
            let serialized = try GSX.serializeJSON(input.markdown, media: media)
            do {
                let typed = try GSX.serialize(input.markdown, media: media)
                guard serialized.hasPrefix(#"{"serialized":"#) else { fatalError("serialize returned for \(serialized)") }
                let wire = try JSONDecoder().decode([String: String].self, from: Data(serialized.utf8))
                guard wire["serialized"] == typed else { fatalError("serialize differs for \(serialized)") }
            } catch let error as ParseError {
                guard serialized.hasPrefix(#"{"error":{"code":"\#(error.code.rawValue)""#) else {
                    fatalError("\(error) for \(serialized)")
                }
            }
            lines.append(serialized)
        }
    }
}
try (lines.joined(separator: "\n") + "\n").write(toFile: out, atomically: true, encoding: .utf8)

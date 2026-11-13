import Foundation
import GSX

struct Case: Decodable {
    struct Item: Decodable {
        let id: String
        let cover: String?
    }

    struct Given: Decodable {
        let media: GSX.MediaKind?
        let mediaCount: Int?
        let placement: GSX.Placement?
        let title: String?
        let targetUrl: String?
        let maxCharacters: UInt32?
        let charactersReservedPerUrl: UInt32?
        let captionLimit: UInt32?
        let descriptionLimit: UInt32?
        let hashtagLimit: UInt32?
        let spoilerText: String?
        let overflow: GSX.Overflow?
        let entitiesAllowed: Bool?
        let items: [GSX.MediaFact]?
    }

    let markdown: String
    let media: [Item]
    let target: String?
    let facts: Given?
}

struct MediaRule: Decodable {
    let mediaRule: Bool?
    let mediaId: String?
}

let targets = try GSX.targets
guard targets.contains(where: { $0.name == "x-plain" }) else { fatalError("targets: \(targets)") }

do {
    let items = [GSX.MediaFact(id: #"x"/><img src="media:evil"#, format: .image)]
    _ = try GSX.projectJSON("", target: "telegram-html-rich", facts: GSX.Facts(items: items))
    fatalError("projectJSON accepts an item ID outside the media ID rule")
} catch {
    guard error.message == "item 0 id is not a media ID" else { fatalError("item ID refusal: \(error)") }
}

let pattern = try NSRegularExpression(pattern: GSX.mediaIdPattern)

func mediaRule(_ record: MediaRule) throws -> String {
    guard let id = record.mediaId else {
        let quoted = String(decoding: try JSONEncoder().encode(GSX.mediaIdPattern), as: UTF8.self)
        return #"{"mediaIdMaxBytes":\#(GSX.mediaIdMaximumBytes),"mediaGroupMin":\#(GSX.mediaGroupMinimumCount),"#
            + #""mediaGroupMax":\#(GSX.mediaGroupMaximumCount),"mediaIdPattern":\#(quoted)}"#
    }
    let whole = NSRange(id.startIndex..., in: id)
    let matched = pattern.firstMatch(in: id, range: whole)?.range == whole
    guard matched == GSX.isMediaId(id) else { fatalError("mediaIdPattern and isMediaId differ for \(id.debugDescription)") }
    return matched ? "true" : "false"
}

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
        if let record = try? decoder.decode(MediaRule.self, from: Data(line.utf8)),
            record.mediaRule != nil || record.mediaId != nil
        {
            lines.append(try mediaRule(record))
            continue
        }
        guard let input = try? decoder.decode(Case.self, from: Data(line.utf8)) else {
            lines.append("-")
            if !line.contains(#""target":"#) { lines.append("-") }
            continue
        }
        let media = input.media.map { GSX.Media(id: $0.id, cover: $0.cover) }
        let given = input.facts
        let facts = GSX.Facts(
            media: given?.media,
            mediaCount: given?.mediaCount ?? 0,
            placement: given?.placement,
            title: given?.title,
            targetUrl: given?.targetUrl,
            maxCharacters: given?.maxCharacters,
            charactersReservedPerUrl: given?.charactersReservedPerUrl,
            captionLimit: given?.captionLimit,
            descriptionLimit: given?.descriptionLimit,
            hashtagLimit: given?.hashtagLimit,
            spoilerText: given?.spoilerText,
            overflow: given?.overflow,
            entitiesAllowed: given?.entitiesAllowed ?? false,
            items: given?.items
        )
        let json: String
        do {
            json = try input.target.map { try GSX.projectJSON(input.markdown, target: $0, facts: facts, media: media) }
                ?? GSX.parseJSON(input.markdown, media: media)
        } catch let error as GSX.InternalError where input.target != nil {
            lines.append(#"{"refused":"\#(error.message)"}"#)
            continue
        }
        do {
            if let target = input.target {
                let projection = try GSX.project(input.markdown, target: target, facts: facts, media: media)
                guard json.hasPrefix(#"{"projection":"#) else { fatalError("project returned a projection for \(json)") }
                guard projection.diagnostics == (try GSX.parse(input.markdown, media: media)).diagnostics else {
                    fatalError("project diagnostics differ from parse for \(json)")
                }
            } else {
                _ = try GSX.parse(input.markdown, media: media)
                guard json.hasPrefix(#"{"document":"#) else { fatalError("parse returned a document for \(json)") }
            }
        } catch let error as GSX.ParseError {
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
            } catch let error as GSX.ParseError {
                guard serialized.hasPrefix(#"{"error":{"code":"\#(error.code.rawValue)""#) else {
                    fatalError("\(error) for \(serialized)")
                }
            }
            lines.append(serialized)
        }
    }
}
try (lines.joined(separator: "\n") + "\n").write(toFile: out, atomically: true, encoding: .utf8)

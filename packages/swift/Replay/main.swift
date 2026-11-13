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
    let targets: Bool?
}

func quoted(_ text: String) -> String {
    var out = "\""
    for scalar in text.unicodeScalars {
        switch scalar {
        case "\"": out += "\\\""
        case "\\": out += "\\\\"
        case "\n": out += "\\n"
        case "\r": out += "\\r"
        case "\t": out += "\\t"
        case "\u{08}": out += "\\b"
        case "\u{0C}": out += "\\f"
        case "\u{00}"..."\u{1F}": out += String(format: "\\u%04x", scalar.value)
        default: out.unicodeScalars.append(scalar)
        }
    }
    return out + "\""
}

func array<T>(_ items: [T], _ each: (T) -> String) -> String {
    "[" + items.map(each).joined(separator: ",") + "]"
}

func optional<T>(_ value: T?, _ each: (T) -> String) -> String {
    value.map(each) ?? "null"
}

func listing() throws -> String {
    func target(_ target: GSX.Target) -> String {
        let fields = array(target.fields) {
            #"{"name":\#(quoted($0.name.rawValue)),"required":\#($0.required),"media":\#(quoted($0.media.rawValue))}"#
        }
        let limits = array(target.limits) { limit in
            #"{"field":\#(quoted(limit.field.rawValue)),"unit":\#(quoted(limit.unit.rawValue)),"#
                + #""maximum":\#(optional(limit.maximum) { String($0) }),"fact":\#(optional(limit.fact, quoted)),"#
                + #""placements":\#(optional(limit.placements) { array($0) { quoted($0.rawValue) } }),"#
                + #""media":\#(quoted(limit.media.rawValue)),"rendering":\#(quoted(limit.rendering.rawValue))}"#
        }
        let features = array(target.features) { rule in
            let conditions = array(rule.conditions) {
                #"{"when":\#(quoted($0.when)),"class":\#(quoted($0.kind.rawValue)),"reason":\#(quoted($0.reason))}"#
            }
            return #"{"feature":\#(quoted(rule.feature.rawValue)),"class":\#(quoted(rule.kind.rawValue)),"#
                + #""reason":\#(quoted(rule.reason)),"conditions":\#(conditions)}"#
        }
        let media = target.media
        let formats = { (formats: [GSX.MediaFact.Format]) in array(formats) { quoted($0.rawValue) } }
        let groups = array(media.groups) {
            #"{"layout":\#(quoted($0.layout.rawValue)),"formats":\#(formats($0.formats)),"#
                + #""minimum":\#($0.minimum),"maximum":\#($0.maximum)}"#
        }
        return #"{"name":\#(quoted(target.name)),"form":\#(quoted(target.form.rawValue)),"#
            + #""offsets":\#(optional(target.offsets) { quoted($0.rawValue) }),"digest":\#(quoted(target.digest)),"#
            + #""manifest":\#(quoted(target.manifest)),"codes":\#(array(target.codes, quoted)),"#
            + #""threads":\#(target.threads),"facts":\#(array(target.facts, quoted)),"fields":\#(fields),"#
            + #""limits":\#(limits),"features":\#(features),"#
            + #""media":{"formats":\#(formats(media.formats)),"alt":\#(formats(media.alt)),"#
            + #""spoiler":{"mode":\#(quoted(media.spoiler.mode.rawValue)),"formats":\#(formats(media.spoiler.formats))},"#
            + #""inline":\#(media.inline),"groups":\#(groups)}}"#
    }
    return #"{"targets":\#(array(try GSX.targets, target))}"#
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
        if let record = try? decoder.decode(MediaRule.self, from: Data(line.utf8)), record.targets == true {
            lines.append(try listing())
            continue
        }
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

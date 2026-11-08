import Foundation
import GSXFFI

public struct Media: Sendable, Hashable {
    public var id: String
    public var cover: String?

    public init(id: String, cover: String? = nil) {
        self.id = id
        self.cover = cover
    }
}

public struct InternalError: Error, Sendable, CustomStringConvertible {
    public let message: String

    public var description: String { message }
}

public enum GSX {
    public static let version = String(cString: gsx_version())

    /// Every target, in the order of the core's table.
    public static let targets: [Target] = {
        let json = try! call("gsx_targets") { error in gsx_targets(&error) }
        return try! JSONDecoder().decode(Listing.self, from: Data(json.utf8)).targets
    }()

    public static func parse(_ text: String, media: [Media] = []) throws -> Document {
        let json = try parseJSON(text, media: media)
        let wire = try JSONDecoder().decode(Wire.self, from: Data(json.utf8))
        if let error = wire.error {
            throw error
        }
        guard let document = wire.document else {
            throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "neither document nor error"))
        }
        return document
    }

    public static func parseJSON(_ text: String, media: [Media] = []) throws(InternalError) -> String {
        var text = text
        return try withMedia(media) { (entries, count) throws(InternalError) -> String in
            try call("gsx_parse") { error in
                text.withUTF8 { utf8 in gsx_parse(utf8.baseAddress, utf8.count, entries, count, &error) }
            }
        }
    }

    /// The canonical `gsx/1` form of the document.
    public static func serialize(_ text: String, media: [Media] = []) throws -> String {
        let json = try serializeJSON(text, media: media)
        let wire = try JSONDecoder().decode(SerializedWire.self, from: Data(json.utf8))
        if let error = wire.error {
            throw error
        }
        guard let serialized = wire.serialized else {
            throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "neither serialized nor error"))
        }
        return serialized
    }

    public static func serializeJSON(_ text: String, media: [Media] = []) throws(InternalError) -> String {
        var text = text
        return try withMedia(media) { (entries, count) throws(InternalError) -> String in
            try call("gsx_serialize") { error in
                text.withUTF8 { utf8 in gsx_serialize(utf8.baseAddress, utf8.count, entries, count, &error) }
            }
        }
    }

    public static func project(
        _ text: String, target: String, facts: Facts = Facts(), media: [Media] = []
    ) throws -> Projection {
        let json = try projectJSON(text, target: target, facts: facts, media: media)
        let wire = try JSONDecoder().decode(ProjectionWire.self, from: Data(json.utf8))
        if let error = wire.error {
            throw error
        }
        guard let projection = wire.projection else {
            throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "neither projection nor error"))
        }
        return projection
    }

    public static func projectJSON(
        _ text: String, target: String, facts: Facts = Facts(), media: [Media] = []
    ) throws(InternalError) -> String {
        guard facts.mediaCount >= 0 else {
            throw InternalError(message: "facts.mediaCount \(facts.mediaCount) is negative")
        }
        var text = text
        var target = target
        var title = facts.title
        var targetUrl = facts.targetUrl
        var spoilerText = facts.spoilerText
        let limit = { (value: UInt32?) in value.map(Int64.init) ?? -1 }
        return try withMedia(media) { (entries, count) throws(InternalError) -> String in
            try withItems(facts.items) { (items, itemCount) throws(InternalError) -> String in
                try call("gsx_project") { error in
                    text.withUTF8 { utf8 in
                        target.withUTF8 { name in
                            withString(&title) { title, titleCount in
                                withString(&targetUrl) { targetUrl, targetUrlCount in
                                    withString(&spoilerText) { spoilerText, spoilerTextCount in
                                        var native = gsx_facts(
                                            media: facts.media?.native ?? UInt32(GSX_MEDIA_NONE),
                                            media_count: facts.mediaCount,
                                            placement: facts.placement?.native ?? UInt32(GSX_PLACEMENT_NONE),
                                            title: title,
                                            title_len: titleCount,
                                            target_url: targetUrl,
                                            target_url_len: targetUrlCount,
                                            max_characters: limit(facts.maxCharacters),
                                            characters_reserved_per_url: limit(facts.charactersReservedPerUrl),
                                            caption_limit: limit(facts.captionLimit),
                                            description_limit: limit(facts.descriptionLimit),
                                            entities_allowed: facts.entitiesAllowed,
                                            items: items,
                                            items_len: itemCount,
                                            hashtag_limit: limit(facts.hashtagLimit),
                                            spoiler_text: spoilerText,
                                            spoiler_text_len: spoilerTextCount,
                                            overflow: facts.overflow?.native ?? UInt32(GSX_OVERFLOW_TRUNCATE)
                                        )
                                        return gsx_project(
                                            utf8.baseAddress, utf8.count, entries, count,
                                            name.baseAddress, name.count, &native, &error
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    private static func withItems<R>(
        _ items: [MediaFact]?, _ body: (UnsafePointer<gsx_media_fact>?, Int) throws(InternalError) -> R
    ) throws(InternalError) -> R {
        guard let items else { return try body(nil, 0) }
        guard !items.isEmpty else {
            let result: Result<R, InternalError> = withUnsafePointer(to: gsx_media_fact()) { empty in
                Result { () throws(InternalError) in try body(empty, 0) }
            }
            return try result.get()
        }
        var storage: [UInt8] = [0]
        var spans: [(id: Range<Int>, alt: Range<Int>?)] = []
        for item in items {
            let id = storage.count
            storage += item.id.utf8
            let idSpan = id..<storage.count
            var altSpan: Range<Int>?
            if let alt = item.alt {
                let start = storage.count
                storage += alt.utf8
                altSpan = start..<storage.count
            }
            spans.append((idSpan, altSpan))
        }
        let result: Result<R, InternalError> = storage.withUnsafeBufferPointer { bytes in
            let base = bytes.baseAddress!
            let entries = zip(items, spans).map { item, span in
                gsx_media_fact(
                    id: base + span.id.lowerBound,
                    id_len: span.id.count,
                    format: item.format.native,
                    alt: span.alt.map { base + $0.lowerBound },
                    alt_len: span.alt?.count ?? 0,
                    spoiler: item.spoiler,
                    inline_: item.inline
                )
            }
            return entries.withUnsafeBufferPointer { entries in
                Result { () throws(InternalError) in try body(entries.baseAddress, entries.count) }
            }
        }
        return try result.get()
    }

    private static func withString<R>(
        _ string: inout String?, _ body: (UnsafePointer<UInt8>?, Int) -> R
    ) -> R {
        guard var value = string else { return body(nil, 0) }
        return value.withUTF8 { utf8 in
            utf8.count == 0 ? withUnsafePointer(to: UInt8(0)) { body($0, 0) } : body(utf8.baseAddress, utf8.count)
        }
    }

    private static func withMedia<R>(
        _ media: [Media], _ body: (UnsafePointer<gsx_media>?, Int) throws(InternalError) -> R
    ) throws(InternalError) -> R {
        var storage: [UInt8] = [0]
        var spans: [(id: Range<Int>, cover: Range<Int>?)] = []
        for item in media {
            let id = storage.count
            storage += item.id.utf8
            let idSpan = id..<storage.count
            var coverSpan: Range<Int>?
            if let cover = item.cover {
                let start = storage.count
                storage += cover.utf8
                coverSpan = start..<storage.count
            }
            spans.append((idSpan, coverSpan))
        }
        let result: Result<R, InternalError> = storage.withUnsafeBufferPointer { bytes in
            let base = bytes.baseAddress!
            let entries = spans.map { span in
                gsx_media(
                    id: base + span.id.lowerBound,
                    id_len: span.id.count,
                    cover: span.cover.map { base + $0.lowerBound },
                    cover_len: span.cover?.count ?? 0
                )
            }
            return entries.withUnsafeBufferPointer { entries in
                Result { () throws(InternalError) in try body(entries.baseAddress, entries.count) }
            }
        }
        return try result.get()
    }

    private static func call(
        _ name: String, _ body: (inout OpaquePointer?) -> OpaquePointer?
    ) throws(InternalError) -> String {
        var error: OpaquePointer?
        guard let output = body(&error) else {
            let message = error.map { String(cString: gsx_error_message($0)) } ?? "\(name) failed"
            gsx_error_free(error)
            throw InternalError(message: message)
        }
        defer { gsx_output_free(output) }
        var len = 0
        let json = gsx_output_json(output, &len)
        return String(decoding: UnsafeRawBufferPointer(start: json, count: len), as: UTF8.self)
    }
}

struct Wire: Decodable {
    let document: Document?
    let error: ParseError?
}

struct SerializedWire: Decodable {
    let serialized: String?
    let error: ParseError?
}

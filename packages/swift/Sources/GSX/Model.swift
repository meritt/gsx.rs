extension GSX {
    public struct Document: Sendable, Hashable, Decodable {
        public let children: [Node]
        public let markers: [Marker]
        public let diagnostics: [Diagnostic]
    }

    public struct Node: Sendable, Hashable, Decodable {
        public let kind: Kind
        public let range: Range<Int>
        public let provenance: Provenance
        public let gapBefore: Int?
        public let children: [Node]

        enum CodingKeys: String, CodingKey {
            case type, range, provenance, gapBefore, children
            case depth, ordered, start, spread, checked, lang, meta, value, align, url, source, title, datetime, unix
            case format, mediaId, mediaIds, layout
        }

        public init(from decoder: any Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            range = try c.decode(Range<Int>.self, forKey: .range)
            provenance = try c.decodeIfPresent(Provenance.self, forKey: .provenance) ?? .authored
            gapBefore = try c.decodeIfPresent(Int.self, forKey: .gapBefore)
            children = try c.decodeIfPresent([Node].self, forKey: .children) ?? []
            func value<T: Decodable>(_ key: CodingKeys) throws -> T { try c.decode(T.self, forKey: key) }
            func optional<T: Decodable>(_ key: CodingKeys) throws -> T? { try c.decodeIfPresent(T.self, forKey: key) }
            kind =
                switch try c.decode(String.self, forKey: .type) {
                case "paragraph": .paragraph
                case "heading": .heading(depth: try value(.depth))
                case "thematicBreak": .thematicBreak
                case "blockquote": .blockquote
                case "list": .list(ordered: try value(.ordered), start: try optional(.start), spread: try value(.spread))
                case "listItem": .listItem(spread: try value(.spread), checked: try optional(.checked))
                case "code": .code(lang: try optional(.lang), meta: try optional(.meta), value: try value(.value))
                case "table": .table(align: try value(.align))
                case "tableRow": .tableRow
                case "tableCell": .tableCell
                case "text": .text(value: try value(.value))
                case "emphasis": .emphasis
                case "strong": .strong
                case "delete": .delete
                case "underline": .underline
                case "highlight": .highlight
                case "spoiler": .spoiler
                case "inlineCode": .inlineCode(value: try value(.value))
                case "math": .math(value: try value(.value))
                case "break": .break
                case "link": .link(url: try value(.url), source: try value(.source), title: try optional(.title))
                case "datetime":
                    .datetime(datetime: try optional(.datetime), unix: try value(.unix), format: try optional(.format))
                case "mediaReference": .mediaReference(id: try value(.mediaId))
                case "mediaGroup": .mediaGroup(ids: try value(.mediaIds), layout: try value(.layout))
                case let type:
                    throw DecodingError.dataCorruptedError(
                        forKey: .type, in: c, debugDescription: "unknown node type \(type)")
                }
        }
    }

    public enum Kind: Sendable, Hashable {
        case paragraph
        case heading(depth: Int)
        case thematicBreak
        case blockquote
        case list(ordered: Bool, start: Int?, spread: Bool)
        case listItem(spread: Bool, checked: Bool?)
        case code(lang: String?, meta: String?, value: String)
        case table(align: [Align?])
        case tableRow
        case tableCell
        case text(value: String)
        case emphasis
        case strong
        case delete
        case underline
        case highlight
        case spoiler
        case inlineCode(value: String)
        case math(value: String)
        case `break`
        case link(url: String, source: String, title: String?)
        case datetime(datetime: String?, unix: Int64, format: TimeFormat?)
        case mediaReference(id: String)
        case mediaGroup(ids: [String], layout: Layout)
    }

    public enum TimeFormat: String, Sendable, Hashable, Decodable {
        case time
        case timeSeconds = "time-seconds"
        case dateShort = "date-short"
        case date
        case datetimeShort = "datetime-short"
        case datetime, full, relative
    }

    public enum Provenance: String, Sendable, Hashable, Decodable {
        case authored, literal
    }

    public enum Align: String, Sendable, Hashable, Decodable {
        case left, right, center
    }

    public enum Layout: String, Sendable, Hashable, Decodable {
        case collage, slideshow
    }

    public struct Marker: Sendable, Hashable, Decodable {
        public let kind: MarkerKind
        public let range: Range<Int>
    }

    public enum MarkerKind: String, Sendable, Hashable, Decodable {
        case emphasis, strong, strikethrough, spoiler, underline, highlight, codeText, codeFence, heading
        case thematicBreak, blockQuote, listItem, taskCheck, tableDelimiter, link, autolink, escape
        case characterReference, hardBreak, directive, media
    }

    public struct Diagnostic: Sendable, Hashable, Decodable {
        public let code: Code
        public let severity: Severity
        public let range: Range<Int>
        public let details: Details
    }

    public enum Code: String, Sendable, Hashable, Decodable {
        case datetimeInvalid = "DATETIME_INVALID"
        case datetimeTimezoneRequired = "DATETIME_TIMEZONE_REQUIRED"
        case linkLiteralized = "LINK_LITERALIZED"
        case markdownInvalidText = "MARKDOWN_INVALID_TEXT"
        case markdownMalformed = "MARKDOWN_MALFORMED"
        case markdownTooComplex = "MARKDOWN_TOO_COMPLEX"
        case mediaDirectiveInvalid = "MEDIA_DIRECTIVE_INVALID"
        case mediaReferenceContextUnsupported = "MEDIA_REFERENCE_CONTEXT_UNSUPPORTED"
        case mediaReferenceCover = "MEDIA_REFERENCE_COVER"
        case mediaReferenceDuplicate = "MEDIA_REFERENCE_DUPLICATE"
        case mediaReferenceUnknown = "MEDIA_REFERENCE_UNKNOWN"
    }

    public enum Severity: String, Sendable, Hashable, Decodable {
        case warning, error
    }

    public enum Scheme: String, Sendable, Hashable, Decodable {
        case http, https, relative, unsupported
    }

    public struct Details: Sendable, Hashable, Decodable {
        public let scheme: Scheme?
        public let id: String?
        public let index: Int?
        public let limit: String?
        public let maximum: Int?
        public let codePoint: UInt32?
    }

    public struct ParseError: Error, Sendable, Hashable, Decodable {
        public let code: Code
        public let range: Range<Int>?
        public let details: Details
    }
}

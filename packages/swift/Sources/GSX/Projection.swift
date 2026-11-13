import GSXFFI

extension GSX {
    public enum MediaKind: String, Sendable, Hashable, Decodable {
        case image, video, gif, audio

        var native: UInt32 {
            switch self {
            case .image: UInt32(GSX_MEDIA_IMAGE)
            case .video: UInt32(GSX_MEDIA_VIDEO)
            case .gif: UInt32(GSX_MEDIA_GIF)
            case .audio: UInt32(GSX_MEDIA_AUDIO)
            }
        }
    }

    /// Selects the limits declared for this placement; without it, or when the target declares none
    /// there, the first of post, reel, story that declares one.
    public enum Placement: String, Sendable, Hashable, Decodable {
        case post, reel, story

        var native: UInt32 {
            switch self {
            case .post: UInt32(GSX_PLACEMENT_POST)
            case .reel: UInt32(GSX_PLACEMENT_REEL)
            case .story: UInt32(GSX_PLACEMENT_STORY)
            }
        }
    }

    /// What a body that does not fit becomes: cut to the limit, or a thread of posts where the target publishes threads.
    public enum Overflow: String, Sendable, Hashable, Decodable {
        case truncate, thread

        var native: UInt32 {
            switch self {
            case .truncate: UInt32(GSX_OVERFLOW_TRUNCATE)
            case .thread: UInt32(GSX_OVERFLOW_THREAD)
            }
        }
    }

    /// An attached media item, matched to media references by `id`.
    public struct MediaFact: Sendable, Hashable, Decodable {
        public enum Format: String, Sendable, Hashable, Decodable {
            case image, gif, video, audio

            var native: UInt32 {
                switch self {
                case .image: UInt32(GSX_FORMAT_IMAGE)
                case .gif: UInt32(GSX_FORMAT_GIF)
                case .video: UInt32(GSX_FORMAT_VIDEO)
                case .audio: UInt32(GSX_FORMAT_AUDIO)
                }
            }
        }

        public var id: String
        public var format: Format
        public var alt: String?
        public var spoiler: Bool
        public var inline: Bool

        public init(id: String, format: Format, alt: String? = nil, spoiler: Bool = false, inline: Bool = false) {
            self.id = id
            self.format = format
            self.alt = alt
            self.spoiler = spoiler
            self.inline = inline
        }
    }

    public struct Facts: Sendable, Hashable {
        public var media: MediaKind?
        public var mediaCount: Int
        public var placement: Placement?
        public var title: String?
        public var targetUrl: String?
        public var maxCharacters: UInt32?
        public var charactersReservedPerUrl: UInt32?
        public var captionLimit: UInt32?
        public var descriptionLimit: UInt32?
        /// Instagram hashtags allowed in a caption; 5 without it.
        public var hashtagLimit: UInt32?
        /// Mastodon content warning: not rendered, but counted with the text against the status limit.
        public var spoilerText: String?
        public var overflow: Overflow?
        public var entitiesAllowed: Bool
        /// The media the caller publishes. A referenced id missing from a given list was dropped; without the list, attaching targets treat every id as attached.
        public var items: [MediaFact]?

        public init(
            media: MediaKind? = nil,
            mediaCount: Int = 0,
            placement: Placement? = nil,
            title: String? = nil,
            targetUrl: String? = nil,
            maxCharacters: UInt32? = nil,
            charactersReservedPerUrl: UInt32? = nil,
            captionLimit: UInt32? = nil,
            descriptionLimit: UInt32? = nil,
            hashtagLimit: UInt32? = nil,
            spoilerText: String? = nil,
            overflow: Overflow? = nil,
            entitiesAllowed: Bool = false,
            items: [MediaFact]? = nil
        ) {
            self.media = media
            self.mediaCount = mediaCount
            self.placement = placement
            self.title = title
            self.targetUrl = targetUrl
            self.maxCharacters = maxCharacters
            self.charactersReservedPerUrl = charactersReservedPerUrl
            self.captionLimit = captionLimit
            self.descriptionLimit = descriptionLimit
            self.hashtagLimit = hashtagLimit
            self.spoilerText = spoilerText
            self.overflow = overflow
            self.entitiesAllowed = entitiesAllowed
            self.items = items
        }
    }

    public struct Projection: Sendable, Hashable, Decodable {
        public let fields: [Field]
        public let measurements: [Measurement]
        public let changes: [Change]
        /// Diagnostics of the parsed document, as in `Document.diagnostics`.
        public let diagnostics: [Diagnostic]
        public let media: [String]
        /// Present for targets of the `blocks/1` form only.
        public let blocks: [Block]?
        /// Media attached to the message with its fitted alt: every media fact item outside the `blocks/1` form, items no block can show on Slack blocks; absent when empty.
        public let items: [Attachment]?
        /// With `overflow: .thread`, when the body did not fit: every post of the thread in order; the top level repeats the first.
        public let parts: [Part]?
    }

    public struct Part: Sendable, Hashable, Decodable {
        public let fields: [Field]
        public let measurements: [Measurement]
        public let media: [String]
        public let items: [Attachment]
        /// UTF-16 range of the part's source in the original input; exact where the rendered text matches its source, otherwise covering and possibly overlapping a neighbor.
        public let range: Range<Int>
        /// Where the split after this part falls; nil on the last part.
        public let boundary: Boundary?
    }

    public enum Boundary: String, Sendable, Hashable, Decodable {
        case paragraph, sentence, word, grapheme
    }

    public struct Attachment: Sendable, Hashable, Decodable {
        public let reference: String
        public let alt: String?
        /// Whether the platform publishes the item hidden: per item, or for the whole post on Mastodon and Threads.
        public let spoiler: Bool
    }

    public struct Item: Sendable, Hashable, Decodable {
        public let reference: String
        public let alt: String?
        public let spoiler: Bool
    }

    public enum Block: Sendable, Hashable, Decodable {
        case text(String)
        case media(items: [Item], layout: Layout?)
        /// An attached file shown in place, for media no media block can show.
        case file(Item)
        case separator

        enum CodingKeys: String, CodingKey {
            case text, items, layout, file, separator
        }

        public init(from decoder: any Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            if let text = try container.decodeIfPresent(String.self, forKey: .text) {
                self = .text(text)
            } else if let file = try container.decodeIfPresent(Item.self, forKey: .file) {
                self = .file(file)
            } else if container.contains(.separator) {
                self = .separator
            } else {
                self = .media(
                    items: try container.decode([Item].self, forKey: .items),
                    layout: try container.decodeIfPresent(Layout.self, forKey: .layout)
                )
            }
        }
    }

    public enum FieldName: String, Sendable, Hashable, Decodable {
        case text, title, description, alt
    }

    public struct Field: Sendable, Hashable, Decodable {
        public let name: FieldName
        public let text: String
        /// UTF-16 ranges of links in `text`.
        public let links: [Range<Int>]
        public let entities: [Entity]
    }

    public enum Style: String, Sendable, Hashable, Decodable {
        case bold, italic, underline, strikethrough, spoiler, code, link, tag, pre, blockquote, date, mention
    }

    public struct Entity: Sendable, Hashable, Decodable {
        public let style: Style
        /// A range of the field text in the target's `offsets` unit.
        public let range: Range<Int>
        public let value: String?
    }

    public enum Unit: String, Sendable, Hashable, Decodable {
        case graphemes
        case codePoints = "code-points"
        case utf16CodeUnits = "utf16-code-units"
        case utf8Bytes = "utf8-bytes"
        case xWeighted = "x-weighted"
        case mastodonWeighted = "mastodon-weighted"
        case threadsWeighted = "threads-weighted"
        case hashtags, mentions, links
    }

    public struct Measurement: Sendable, Hashable, Decodable {
        public let field: FieldName
        public let unit: Unit
        public let value: Int
        public let maximum: Int?
        /// Per-item limits: the block and the item in it on blocks targets, else the index in `items`.
        public let block: Int?
        public let item: Int?
    }

    public enum ChangeClass: String, Sendable, Hashable, Decodable {
        case approximated
        case textualFallback = "textual_fallback"
        case unwrapped, omitted, blocked
    }

    public enum Feature: String, Sendable, Hashable, Decodable {
        case heading
        case thematicBreak = "thematic_break"
        case blockquote
        case bulletList = "bullet_list"
        case orderedList = "ordered_list"
        case compoundList = "compound_list"
        case codeBlock = "code_block"
        case table
        case taskList = "task_list"
        case media, emphasis, strong, strikethrough, underline, highlight, spoiler
        case inlineCode = "inline_code"
        case link
        case linkTitle = "link_title"
        case datetime
        case angleBrackets = "angle_brackets"
        case text, title
        case targetUrl = "target_url"
        case hashtag, mention
    }

    public struct Truncated: Sendable, Hashable, Decodable {
        public let unit: Unit
        public let maximum: Int
        public let value: Int
        public let exact: Bool
    }

    public struct Change: Sendable, Hashable, Decodable {
        public let kind: ChangeClass
        public let feature: Feature
        public let reason: String
        /// The reason in upper case when `kind` is `.blocked`.
        public let code: String?
        public let field: FieldName
        public let range: Range<Int>?
        public let truncated: Truncated?
        /// Alt and spoiler changes: the block and the item in it on blocks targets, else the index in `items`.
        public let block: Int?
        public let item: Int?

        enum CodingKeys: String, CodingKey {
            case kind = "class", feature, reason, code, field, range, truncated, block, item
        }
    }

    public struct Target: Sendable, Hashable, Decodable {
        public enum Form: String, Sendable, Hashable, Decodable {
            case plain = "plain/1"
            case entities = "entities/1"
            case html = "html/1"
            case markdown = "markdown/1"
            case blocks = "blocks/1"
        }

        public enum Offsets: String, Sendable, Hashable, Decodable {
            case utf8, utf16
        }

        public let name: String
        public let form: Form
        public let offsets: Offsets?
        public let digest: String
        public let manifest: String
        /// Every code a blocked change of this target can carry.
        public let codes: [String]
        /// Whether `overflow: .thread` applies.
        public let threads: Bool
        /// Facts that can change this target's projection, in their wire spelling, such as `maxCharacters`.
        public let facts: [String]
        public let fields: [TargetField]
        /// At most one limit applies to one field, unit, placement and media kind.
        public let limits: [Limit]
        public let features: [Rule]
        public let media: MediaCapabilities
    }

    public struct TargetField: Sendable, Hashable, Decodable {
        public enum Media: String, Sendable, Hashable, Decodable {
            case any, image, video
        }

        public let name: FieldName
        public let required: Bool
        /// The media kind the field needs.
        public let media: Media
    }

    /// A fixed limit has `maximum` and no `fact`; a fact with a default has both; a fact without a default has `fact` only; an unlimited count has neither.
    public struct Limit: Sendable, Hashable, Decodable {
        public enum Media: String, Sendable, Hashable, Decodable {
            case any, with, without, image, video, gif, audio
        }

        public enum Rendering: String, Sendable, Hashable, Decodable {
            case payload, visible
        }

        public let field: FieldName
        public let unit: Unit
        public let maximum: Int?
        /// The fact in its wire spelling, such as `maxCharacters`.
        public let fact: String?
        /// `nil` means every placement.
        public let placements: [Placement]?
        public let media: Media
        public let rendering: Rendering
    }

    /// A feature's default outcome and the conditional outcomes that replace it.
    public struct Rule: Sendable, Hashable, Decodable {
        public struct Condition: Sendable, Hashable, Decodable {
            public let when: String
            public let kind: ChangeClass
            public let reason: String

            enum CodingKeys: String, CodingKey {
                case when, kind = "class", reason
            }
        }

        public let feature: Feature
        public let kind: ChangeClass
        public let reason: String
        public let conditions: [Condition]

        enum CodingKeys: String, CodingKey {
            case feature, kind = "class", reason, conditions
        }
    }

    public struct MediaCapabilities: Sendable, Hashable, Decodable {
        public enum SpoilerMode: String, Sendable, Hashable, Decodable {
            case carried, widened, omitted
        }

        public struct Spoiler: Sendable, Hashable, Decodable {
            public let mode: SpoilerMode
            public let formats: [MediaFact.Format]
        }

        public struct Group: Sendable, Hashable, Decodable {
            public let layout: Layout
            /// Item formats placed in a group of this layout.
            public let formats: [MediaFact.Format]
            public let minimum: Int
            public let maximum: Int
        }

        /// Item formats published as media; where `inline` is read, only inline items of these formats are placed.
        public let formats: [MediaFact.Format]
        /// Item formats whose alt the projection publishes.
        public let alt: [MediaFact.Format]
        public let spoiler: Spoiler
        /// Whether `MediaFact.inline` is read.
        public let inline: Bool
        /// Group layouts rendered as a group; empty when none.
        public let groups: [Group]
    }

    struct ProjectionWire: Decodable {
        let projection: Projection?
        let error: ParseError?
    }

    struct Listing: Decodable {
        let targets: [Target]
    }
}

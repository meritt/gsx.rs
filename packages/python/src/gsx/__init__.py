from __future__ import annotations

import ctypes
import json
import os
from dataclasses import dataclass
from typing import Iterable, Literal, NamedTuple, Optional, Sequence, Tuple

__all__ = [
    "__version__",
    "MEDIA_GROUP_MAX",
    "MEDIA_GROUP_MIN",
    "MEDIA_ID_MAX_BYTES",
    "MEDIA_ID_PATTERN",
    "Attachment",
    "Block",
    "Change",
    "Condition",
    "Details",
    "Diagnostic",
    "Document",
    "Entity",
    "Facts",
    "Field",
    "Group",
    "InternalError",
    "Item",
    "Limit",
    "Marker",
    "Measurement",
    "Part",
    "Media",
    "MediaCapabilities",
    "MediaFact",
    "Node",
    "ParseError",
    "Projection",
    "Range",
    "Rule",
    "Spoiler",
    "Target",
    "TargetField",
    "Truncated",
    "is_media_id",
    "parse",
    "parse_json",
    "project",
    "project_json",
    "serialize",
    "serialize_json",
    "targets",
]

Align = Literal["left", "right", "center"]
Layout = Literal["collage", "slideshow"]
Provenance = Literal["authored", "literal"]
TimeFormat = Literal["time", "time-seconds", "date-short", "date", "datetime-short", "datetime", "full", "relative"]
NodeType = Literal[
    "paragraph",
    "heading",
    "thematicBreak",
    "blockquote",
    "list",
    "listItem",
    "code",
    "table",
    "tableRow",
    "tableCell",
    "text",
    "emphasis",
    "strong",
    "delete",
    "underline",
    "highlight",
    "spoiler",
    "inlineCode",
    "math",
    "break",
    "link",
    "datetime",
    "mediaReference",
    "mediaGroup",
]
MarkerKind = Literal[
    "emphasis",
    "strong",
    "strikethrough",
    "spoiler",
    "underline",
    "highlight",
    "codeText",
    "codeFence",
    "heading",
    "thematicBreak",
    "blockQuote",
    "listItem",
    "taskCheck",
    "tableDelimiter",
    "link",
    "autolink",
    "escape",
    "characterReference",
    "hardBreak",
    "directive",
    "media",
]
Code = Literal[
    "DATETIME_INVALID",
    "DATETIME_TIMEZONE_REQUIRED",
    "LINK_LITERALIZED",
    "MARKDOWN_INVALID_TEXT",
    "MARKDOWN_MALFORMED",
    "MARKDOWN_TOO_COMPLEX",
    "MEDIA_DIRECTIVE_INVALID",
    "MEDIA_REFERENCE_CONTEXT_UNSUPPORTED",
    "MEDIA_REFERENCE_COVER",
    "MEDIA_REFERENCE_DUPLICATE",
    "MEDIA_REFERENCE_UNKNOWN",
]
Severity = Literal["warning", "error"]
Scheme = Literal["http", "https", "relative", "unsupported"]
ComplexityLimit = Literal[
    "blockDelimiters",
    "blockLabels",
    "blockLabelEnds",
    "containerDepth",
    "documentEscapes",
    "documentHardBreaks",
    "documentLines",
    "documentListItems",
    "documentMathLiterals",
    "documentPairedDelimiters",
    "documentTableCells",
    "documentUnits",
    "inlineDepth",
]
MediaKind = Literal["image", "video", "gif", "audio"]
MediaFormat = Literal["image", "gif", "video", "audio"]
Placement = Literal["post", "reel", "story"]
Overflow = Literal["truncate", "thread"]
FieldName = Literal["text", "title", "description", "alt"]
Style = Literal[
    "bold",
    "italic",
    "underline",
    "strikethrough",
    "spoiler",
    "code",
    "link",
    "tag",
    "pre",
    "blockquote",
    "date",
    "mention",
]
EntityFormat = Literal["t", "T", "d", "D", "dt", "Dt", "wDt", "r"]
Unit = Literal[
    "graphemes",
    "code-points",
    "utf16-code-units",
    "utf8-bytes",
    "x-weighted",
    "mastodon-weighted",
    "threads-weighted",
    "hashtags",
    "mentions",
    "links",
]
ChangeClass = Literal["approximated", "textual_fallback", "unwrapped", "omitted", "blocked"]
Feature = Literal[
    "heading",
    "thematic_break",
    "blockquote",
    "bullet_list",
    "ordered_list",
    "compound_list",
    "code_block",
    "table",
    "task_list",
    "media",
    "emphasis",
    "strong",
    "strikethrough",
    "underline",
    "highlight",
    "spoiler",
    "inline_code",
    "math",
    "link",
    "link_title",
    "datetime",
    "angle_brackets",
    "text",
    "title",
    "target_url",
    "hashtag",
    "mention",
]
Boundary = Literal["paragraph", "sentence", "word", "grapheme"]
TargetForm = Literal["plain/1", "entities/1", "html/1", "markdown/1", "blocks/1"]
TargetOffsets = Literal["utf8", "utf16"]
FactName = Literal[
    "media",
    "mediaCount",
    "placement",
    "title",
    "targetUrl",
    "maxCharacters",
    "charactersReservedPerUrl",
    "captionLimit",
    "descriptionLimit",
    "hashtagLimit",
    "spoilerText",
    "overflow",
    "entitiesAllowed",
    "items",
]
TargetFieldMedia = Literal["any", "image", "video"]
LimitMedia = Literal["any", "with", "without", "image", "video", "gif", "audio"]
LimitRendering = Literal["payload", "visible"]
SpoilerMode = Literal["carried", "widened", "omitted"]
When = Literal[
    "label_is_href",
    "label_invisible",
    "title_source",
    "empty",
    "overflow",
    "capped",
    "nested",
    "info",
    "blocks",
    "depth",
    "columns",
    "group",
    "cut",
    "inline",
    "missing",
    "count",
    "spoiler",
    "truncated",
    "refused",
    "thread",
    "code",
    "post",
    "blank_lines",
]


def _load() -> ctypes.CDLL:
    here = os.path.dirname(os.path.abspath(__file__))
    for name in ("libgsx.dylib", "libgsx.so"):
        path = os.path.join(here, name)
        if os.path.exists(path):
            return ctypes.CDLL(path)
    raise ImportError("gsx: the native library is missing from " + here)


class _Media(ctypes.Structure):
    _fields_ = [
        ("id", ctypes.c_char_p),
        ("id_len", ctypes.c_size_t),
        ("cover", ctypes.c_char_p),
        ("cover_len", ctypes.c_size_t),
    ]


_lib = _load()
_lib.gsx_parse.argtypes = [
    ctypes.c_char_p,
    ctypes.c_size_t,
    ctypes.POINTER(_Media),
    ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_void_p),
]
_lib.gsx_parse.restype = ctypes.c_void_p
_lib.gsx_serialize.argtypes = _lib.gsx_parse.argtypes
_lib.gsx_serialize.restype = ctypes.c_void_p
_lib.gsx_output_json.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t)]
_lib.gsx_output_json.restype = ctypes.c_void_p
_lib.gsx_output_free.argtypes = [ctypes.c_void_p]
_lib.gsx_output_free.restype = None
_lib.gsx_error_message.argtypes = [ctypes.c_void_p]
_lib.gsx_error_message.restype = ctypes.c_char_p
_lib.gsx_error_free.argtypes = [ctypes.c_void_p]
_lib.gsx_error_free.restype = None
_lib.gsx_version.argtypes = []
_lib.gsx_version.restype = ctypes.c_char_p
_lib.gsx_is_media_id.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
_lib.gsx_is_media_id.restype = ctypes.c_bool
_lib.gsx_media_id_pattern.argtypes = []
_lib.gsx_media_id_pattern.restype = ctypes.c_char_p
for _bound in (_lib.gsx_media_id_max_bytes, _lib.gsx_media_group_min, _lib.gsx_media_group_max):
    _bound.argtypes = []
    _bound.restype = ctypes.c_size_t

__version__: str = _lib.gsx_version().decode()
MEDIA_ID_MAX_BYTES: int = _lib.gsx_media_id_max_bytes()
MEDIA_GROUP_MIN: int = _lib.gsx_media_group_min()
MEDIA_GROUP_MAX: int = _lib.gsx_media_group_max()
MEDIA_ID_PATTERN: str = _lib.gsx_media_id_pattern().decode()


class Range(NamedTuple):
    start: int
    end: int


@dataclass(frozen=True)
class Media:
    id: str
    cover: Optional[str] = None


@dataclass(frozen=True)
class Details:
    scheme: Optional[Scheme] = None
    id: Optional[str] = None
    index: Optional[int] = None
    limit: Optional[ComplexityLimit] = None
    maximum: Optional[int] = None
    code_point: Optional[int] = None


@dataclass(frozen=True)
class Node:
    type: NodeType
    range: Range
    provenance: Provenance = "authored"
    gap_before: Optional[int] = None
    children: Tuple["Node", ...] = ()
    depth: Optional[int] = None
    ordered: Optional[bool] = None
    start: Optional[int] = None
    spread: Optional[bool] = None
    checked: Optional[bool] = None
    lang: Optional[str] = None
    meta: Optional[str] = None
    value: Optional[str] = None
    align: Optional[Tuple[Optional[Align], ...]] = None
    url: Optional[str] = None
    source: Optional[str] = None
    title: Optional[str] = None
    datetime: Optional[str] = None
    unix: Optional[int] = None
    format: Optional[TimeFormat] = None
    media_id: Optional[str] = None
    media_ids: Optional[Tuple[str, ...]] = None
    layout: Optional[Layout] = None


@dataclass(frozen=True)
class Marker:
    kind: MarkerKind
    range: Range


@dataclass(frozen=True)
class Diagnostic:
    code: Code
    severity: Severity
    range: Range
    details: Details


@dataclass(frozen=True)
class Document:
    children: Tuple[Node, ...]
    markers: Tuple[Marker, ...]
    diagnostics: Tuple[Diagnostic, ...]


class ParseError(Exception):
    def __init__(self, code: Code, range: Optional[Range], details: Details) -> None:
        super().__init__(code if range is None else "%s at %d..%d" % (code, range.start, range.end))
        self.code = code
        self.range = range
        self.details = details


class InternalError(Exception):
    pass


_KEYS = {"gapBefore": "gap_before", "mediaId": "media_id", "mediaIds": "media_ids", "codePoint": "code_point"}


def _details(value: dict) -> Details:
    return Details(**{_KEYS.get(key, key): item for key, item in value.items()})


def _node(value: dict) -> Node:
    fields = {_KEYS.get(key, key): item for key, item in value.items()}
    fields["range"] = Range(*value["range"])
    fields["children"] = tuple(_node(child) for child in value.get("children", ()))
    for key in ("align", "media_ids"):
        if fields.get(key) is not None:
            fields[key] = tuple(fields[key])
    return Node(**fields)


def _encode(text: str) -> bytes:
    try:
        return text.encode("utf-8")
    except UnicodeEncodeError:
        paired = text.encode("utf-16-le", "surrogatepass").decode("utf-16-le", "surrogatepass")
        return paired.encode("utf-8", "surrogatepass")


def _utf8(value: object, name: str) -> bytes:
    if not isinstance(value, str):
        raise TypeError(name + " must be a str")
    try:
        return value.encode("utf-8")
    except UnicodeEncodeError:
        raise TypeError(name + " must be a well-formed str") from None


def _opt_utf8(value: object, name: str) -> Optional[bytes]:
    if value is None:
        return None
    return _utf8(value, name)


def _flag(value: object, name: str) -> int:
    if not isinstance(value, bool):
        raise TypeError(name + " must be a bool")
    return 1 if value else 0


def _media(media: Iterable[Media]) -> ctypes.Array:
    entries = []
    for item in media:
        ident = _utf8(item.id, "media.id")
        cover = _opt_utf8(item.cover, "media.cover")
        entries.append(_Media(ident, len(ident), cover, 0 if cover is None else len(cover)))
    return (_Media * len(entries))(*entries)


def _call(name: str, call) -> str:
    error = ctypes.c_void_p()
    output = call(ctypes.byref(error))
    if not output:
        try:
            message = _lib.gsx_error_message(error).decode("utf-8", "replace") if error else name + " failed"
        finally:
            _lib.gsx_error_free(error)
        raise InternalError(message)
    try:
        size = ctypes.c_size_t()
        pointer = _lib.gsx_output_json(output, ctypes.byref(size))
        return ctypes.string_at(pointer, size.value).decode("utf-8")
    finally:
        _lib.gsx_output_free(output)


def _text(text: str) -> bytes:
    if not isinstance(text, str):
        raise TypeError("text must be a str")
    return _encode(text)


def is_media_id(id: str) -> bool:
    if not isinstance(id, str):
        raise TypeError("id must be a str")
    if len(id) > MEDIA_ID_MAX_BYTES:
        return False
    data = _encode(id)
    return _lib.gsx_is_media_id(data, len(data))


def parse_json(text: str, media: Iterable[Media] = ()) -> str:
    data = _text(text)
    array = _media(media)
    return _call("gsx_parse", lambda error: _lib.gsx_parse(data, len(data), array, len(array), error))


def serialize_json(text: str, media: Iterable[Media] = ()) -> str:
    data = _text(text)
    array = _media(media)
    return _call("gsx_serialize", lambda error: _lib.gsx_serialize(data, len(data), array, len(array), error))


def serialize(text: str, media: Iterable[Media] = ()) -> str:
    return _unwrap(serialize_json(text, media), "serialized")


def _unwrap(raw: str, key: str):
    wire = json.loads(raw)
    if "error" in wire:
        error = wire["error"]
        span = error["range"]
        raise ParseError(error["code"], None if span is None else Range(*span), _details(error["details"]))
    return wire[key]


def parse(text: str, media: Iterable[Media] = ()) -> Document:
    document = _unwrap(parse_json(text, media), "document")
    return Document(
        children=tuple(_node(child) for child in document["children"]),
        markers=tuple(Marker(item["kind"], Range(*item["range"])) for item in document["markers"]),
        diagnostics=_diagnostics(document["diagnostics"]),
    )


def _diagnostics(diagnostics: list) -> Tuple[Diagnostic, ...]:
    return tuple(
        Diagnostic(item["code"], item["severity"], Range(*item["range"]), _details(item["details"]))
        for item in diagnostics
    )


@dataclass(frozen=True)
class MediaFact:
    """An attached media item, matched to media references by id."""

    id: str
    format: MediaFormat
    alt: Optional[str] = None
    spoiler: bool = False
    inline: bool = False


@dataclass(frozen=True, kw_only=True)
class Facts:
    media: Optional[MediaKind] = None
    media_count: int = 0
    placement: Optional[Placement] = None
    title: Optional[str] = None
    target_url: Optional[str] = None
    max_characters: Optional[int] = None
    characters_reserved_per_url: Optional[int] = None
    caption_limit: Optional[int] = None
    description_limit: Optional[int] = None
    hashtag_limit: Optional[int] = None
    """Instagram hashtags allowed in a caption; 5 without it."""
    spoiler_text: Optional[str] = None
    """Mastodon content warning: not rendered, but counted with the text against the status limit."""
    overflow: Optional[Overflow] = None
    """What a body that does not fit becomes: "truncate" (the default) or "thread" where the target publishes threads."""
    entities_allowed: bool = False
    items: Optional[Sequence[MediaFact]] = None


@dataclass(frozen=True)
class Entity:
    """A styled range of the field text in the target's offsets unit."""

    style: Style
    range: Range
    value: Optional[str]
    format: Optional[EntityFormat] = None


@dataclass(frozen=True)
class Field:
    name: FieldName
    text: str
    links: Tuple[Range, ...]
    entities: Tuple[Entity, ...]


@dataclass(frozen=True)
class Measurement:
    field: FieldName
    unit: Unit
    value: int
    maximum: Optional[int]
    block: Optional[int] = None
    """Per-item limits: the block and the item in it on blocks targets, else the index in items."""
    item: Optional[int] = None


@dataclass(frozen=True)
class Attachment:
    """A media item as published: alt fitted, and hidden when the platform carries its spoiler."""

    reference: str
    alt: Optional[str]
    spoiler: bool


@dataclass(frozen=True)
class Item:
    reference: str
    alt: Optional[str]
    spoiler: bool


@dataclass(frozen=True)
class Block:
    """Text, media items with an optional layout, an attached file shown in place, or a separator."""

    text: Optional[str] = None
    items: Optional[Tuple[Item, ...]] = None
    layout: Optional[Layout] = None
    file: Optional[Item] = None
    separator: bool = False


@dataclass(frozen=True)
class Truncated:
    unit: Unit
    maximum: int
    value: int
    exact: bool


@dataclass(frozen=True)
class Change:
    class_: ChangeClass
    feature: Feature
    reason: str
    code: Optional[str]
    field: FieldName
    range: Optional[Range]
    truncated: Optional[Truncated]
    block: Optional[int] = None
    item: Optional[int] = None


@dataclass(frozen=True)
class Projection:
    fields: Tuple[Field, ...]
    measurements: Tuple[Measurement, ...]
    changes: Tuple[Change, ...]
    diagnostics: Tuple[Diagnostic, ...]
    """Diagnostics of the parsed document, as in Document.diagnostics."""
    media: Tuple[str, ...]
    blocks: Optional[Tuple[Block, ...]] = None
    """Present for targets of the blocks/1 form only."""
    items: Optional[Tuple[Attachment, ...]] = None
    """Media attached to the message with its fitted alt: every media fact item outside the blocks/1 form, items no block can show on Slack blocks; absent when empty."""
    parts: Optional[Tuple["Part", ...]] = None
    """With overflow "thread", when the body did not fit: every post of the thread in order; the top level repeats the first."""


@dataclass(frozen=True)
class Part:
    fields: Tuple[Field, ...]
    measurements: Tuple[Measurement, ...]
    media: Tuple[str, ...]
    items: Tuple[Attachment, ...]
    range: Range
    boundary: Optional[Boundary]


@dataclass(frozen=True)
class TargetField:
    name: FieldName
    required: bool
    media: TargetFieldMedia
    """The media kind the field needs: "any", "image" or "video"."""


@dataclass(frozen=True)
class Limit:
    """A fixed limit has maximum and no fact; a fact with a default has both; a fact without a default has fact only; an unlimited count has neither."""

    field: FieldName
    unit: Unit
    maximum: Optional[int]
    fact: Optional[FactName]
    """The fact in its wire spelling, such as "maxCharacters"."""
    placements: Optional[Tuple[Placement, ...]]
    """None means every placement."""
    media: LimitMedia
    rendering: LimitRendering


@dataclass(frozen=True)
class Condition:
    when: When
    class_: ChangeClass
    reason: str


@dataclass(frozen=True)
class Rule:
    """A feature's default outcome and the conditional outcomes that replace it."""

    feature: Feature
    class_: ChangeClass
    reason: str
    conditions: Tuple[Condition, ...]


@dataclass(frozen=True)
class Group:
    layout: Layout
    formats: Tuple[MediaFormat, ...]
    minimum: int
    maximum: int


@dataclass(frozen=True)
class Spoiler:
    mode: SpoilerMode
    """"carried", "widened" or "omitted"."""
    formats: Tuple[MediaFormat, ...]


@dataclass(frozen=True)
class MediaCapabilities:
    formats: Tuple[MediaFormat, ...]
    """Item formats published as media; where inline is read, only inline items of these formats are placed."""
    alt: Tuple[MediaFormat, ...]
    """Item formats whose alt the projection publishes."""
    spoiler: Spoiler
    inline: bool
    """Whether items[].inline is read."""
    groups: Tuple[Group, ...]
    """Group layouts rendered as a group; empty when none."""


@dataclass(frozen=True)
class Target:
    name: str
    form: TargetForm
    offsets: Optional[TargetOffsets]
    digest: str
    manifest: str
    codes: Tuple[str, ...]
    """Every code a blocked change of this target can carry."""
    threads: bool
    """Whether overflow "thread" applies."""
    facts: Tuple[FactName, ...]
    """Facts that can change this target's projection, in their wire spelling, such as "maxCharacters"."""
    fields: Tuple[TargetField, ...]
    limits: Tuple[Limit, ...]
    """At most one limit applies to one field, unit, placement and media kind."""
    features: Tuple[Rule, ...]
    media: MediaCapabilities


class _MediaFact(ctypes.Structure):
    _fields_ = [
        ("id", ctypes.c_char_p),
        ("id_len", ctypes.c_size_t),
        ("format", ctypes.c_uint32),
        ("alt", ctypes.c_char_p),
        ("alt_len", ctypes.c_size_t),
        ("spoiler", ctypes.c_uint8),
        ("inline", ctypes.c_uint8),
    ]


class _Facts(ctypes.Structure):
    _fields_ = [
        ("media", ctypes.c_uint32),
        ("media_count", ctypes.c_size_t),
        ("placement", ctypes.c_uint32),
        ("title", ctypes.c_char_p),
        ("title_len", ctypes.c_size_t),
        ("target_url", ctypes.c_char_p),
        ("target_url_len", ctypes.c_size_t),
        ("max_characters", ctypes.c_int64),
        ("characters_reserved_per_url", ctypes.c_int64),
        ("caption_limit", ctypes.c_int64),
        ("description_limit", ctypes.c_int64),
        ("entities_allowed", ctypes.c_uint8),
        ("items", ctypes.POINTER(_MediaFact)),
        ("items_len", ctypes.c_size_t),
        ("hashtag_limit", ctypes.c_int64),
        ("spoiler_text", ctypes.c_char_p),
        ("spoiler_text_len", ctypes.c_size_t),
        ("overflow", ctypes.c_uint32),
    ]


_lib.gsx_project.argtypes = [
    ctypes.c_char_p,
    ctypes.c_size_t,
    ctypes.POINTER(_Media),
    ctypes.c_size_t,
    ctypes.c_char_p,
    ctypes.c_size_t,
    ctypes.POINTER(_Facts),
    ctypes.POINTER(ctypes.c_void_p),
]
_lib.gsx_project.restype = ctypes.c_void_p
_lib.gsx_targets.argtypes = [ctypes.POINTER(ctypes.c_void_p)]
_lib.gsx_targets.restype = ctypes.c_void_p

_KINDS = {None: 0, "image": 1, "video": 2, "gif": 3, "audio": 4}
_PLACEMENTS = {None: 0, "post": 1, "reel": 2, "story": 3}
_OVERFLOWS = {None: 0, "truncate": 0, "thread": 1}
_FORMATS = {"image": 0, "gif": 1, "video": 2, "audio": 3}


def _limit(value: Optional[int], name: str) -> int:
    if value is None:
        return -1
    if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 0xFFFFFFFF:
        raise ValueError(name + " must be an int from 0 to 2**32 - 1 or None")
    return value


def _facts(facts: Facts) -> _Facts:
    if facts.media not in _KINDS:
        raise ValueError("facts.media must be 'image', 'video', 'gif', 'audio' or None")
    if facts.placement not in _PLACEMENTS:
        raise ValueError("facts.placement must be 'post', 'reel', 'story' or None")
    if facts.overflow not in _OVERFLOWS:
        raise ValueError("facts.overflow must be 'truncate', 'thread' or None")
    count = facts.media_count
    if not isinstance(count, int) or isinstance(count, bool) or not 0 <= count < 1 << 8 * ctypes.sizeof(ctypes.c_size_t):
        raise ValueError("facts.media_count must be a non-negative int that fits size_t")
    title = _opt_utf8(facts.title, "facts.title")
    target_url = _opt_utf8(facts.target_url, "facts.target_url")
    spoiler_text = _opt_utf8(facts.spoiler_text, "facts.spoiler_text")
    entities_allowed = _flag(facts.entities_allowed, "facts.entities_allowed")
    listed = () if facts.items is None else tuple(facts.items)
    items = None if facts.items is None else (_MediaFact * len(listed))()
    for index, item in enumerate(listed):
        if item.format not in _FORMATS:
            raise ValueError("facts.items format must be 'image', 'gif', 'video' or 'audio'")
        id_ = _utf8(item.id, "facts.items.id")
        alt = _opt_utf8(item.alt, "facts.items.alt")
        items[index] = _MediaFact(
            id_,
            len(id_),
            _FORMATS[item.format],
            alt,
            0 if alt is None else len(alt),
            _flag(item.spoiler, "facts.items.spoiler"),
            _flag(item.inline, "facts.items.inline"),
        )
    return _Facts(
        _KINDS[facts.media],
        facts.media_count,
        _PLACEMENTS[facts.placement],
        title,
        0 if title is None else len(title),
        target_url,
        0 if target_url is None else len(target_url),
        _limit(facts.max_characters, "facts.max_characters"),
        _limit(facts.characters_reserved_per_url, "facts.characters_reserved_per_url"),
        _limit(facts.caption_limit, "facts.caption_limit"),
        _limit(facts.description_limit, "facts.description_limit"),
        entities_allowed,
        items,
        len(listed),
        _limit(facts.hashtag_limit, "facts.hashtag_limit"),
        spoiler_text,
        0 if spoiler_text is None else len(spoiler_text),
        _OVERFLOWS[facts.overflow],
    )


def project_json(text: str, target: str, facts: Facts = Facts(), media: Iterable[Media] = ()) -> str:
    data = _text(text)
    name = _utf8(target, "target")
    native = _facts(facts)
    array = _media(media)
    return _call(
        "gsx_project",
        lambda error: _lib.gsx_project(
            data, len(data), array, len(array), name, len(name), ctypes.byref(native), error
        ),
    )


def _range(value: Optional[list]) -> Optional[Range]:
    return None if value is None else Range(*value)


def project(text: str, target: str, facts: Facts = Facts(), media: Iterable[Media] = ()) -> Projection:
    projection = _unwrap(project_json(text, target, facts, media), "projection")
    return Projection(
        fields=_fields(projection["fields"]),
        measurements=_measurements(projection["measurements"]),
        changes=tuple(
            Change(
                item["class"],
                item["feature"],
                item["reason"],
                item["code"],
                item["field"],
                _range(item["range"]),
                None if item["truncated"] is None else Truncated(**item["truncated"]),
                item.get("block"),
                item.get("item"),
            )
            for item in projection["changes"]
        ),
        diagnostics=_diagnostics(projection["diagnostics"]),
        media=tuple(projection["media"]),
        blocks=None if "blocks" not in projection else tuple(_block(block) for block in projection["blocks"]),
        items=None if "items" not in projection else tuple(Attachment(**item) for item in projection["items"]),
        parts=None if "parts" not in projection else tuple(_part(part) for part in projection["parts"]),
    )


def _fields(fields: list) -> Tuple[Field, ...]:
    return tuple(
        Field(
            item["name"],
            item["text"],
            tuple(Range(*link) for link in item["links"]),
            tuple(
                Entity(entity["style"], Range(*entity["range"]), entity["value"], entity.get("format"))
                for entity in item["entities"]
            ),
        )
        for item in fields
    )


def _measurements(measurements: list) -> Tuple[Measurement, ...]:
    return tuple(
        Measurement(item["field"], item["unit"], item["value"], item["maximum"], item.get("block"), item.get("item"))
        for item in measurements
    )


def _part(part: dict) -> Part:
    return Part(
        _fields(part["fields"]),
        _measurements(part["measurements"]),
        tuple(part["media"]),
        tuple(Attachment(**item) for item in part["items"]),
        Range(*part["range"]),
        part["boundary"],
    )


def _block(block: dict) -> Block:
    if "text" in block:
        return Block(text=block["text"])
    if "file" in block:
        return Block(file=Item(**block["file"]))
    if "separator" in block:
        return Block(separator=True)
    return Block(items=tuple(Item(**item) for item in block["items"]), layout=block["layout"])


_TARGETS: Optional[Tuple[Target, ...]] = None


def targets() -> Tuple[Target, ...]:
    global _TARGETS
    if _TARGETS is None:
        listing = json.loads(_call("gsx_targets", _lib.gsx_targets))
        _TARGETS = tuple(_target(item) for item in listing["targets"])
    return _TARGETS


def _target(item: dict) -> Target:
    media = item["media"]
    return Target(
        item["name"],
        item["form"],
        item["offsets"],
        item["digest"],
        item["manifest"],
        tuple(item["codes"]),
        item["threads"],
        tuple(item["facts"]),
        tuple(TargetField(**spec) for spec in item["fields"]),
        tuple(
            Limit(
                limit["field"],
                limit["unit"],
                limit["maximum"],
                limit["fact"],
                None if limit["placements"] is None else tuple(limit["placements"]),
                limit["media"],
                limit["rendering"],
            )
            for limit in item["limits"]
        ),
        tuple(
            Rule(
                rule["feature"],
                rule["class"],
                rule["reason"],
                tuple(
                    Condition(condition["when"], condition["class"], condition["reason"])
                    for condition in rule["conditions"]
                ),
            )
            for rule in item["features"]
        ),
        MediaCapabilities(
            tuple(media["formats"]),
            tuple(media["alt"]),
            Spoiler(media["spoiler"]["mode"], tuple(media["spoiler"]["formats"])),
            media["inline"],
            tuple(
                Group(group["layout"], tuple(group["formats"]), group["minimum"], group["maximum"])
                for group in media["groups"]
            ),
        ),
    )

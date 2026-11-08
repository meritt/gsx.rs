from __future__ import annotations

import ctypes
import json
import os
from dataclasses import dataclass, field
from typing import Iterable, List, NamedTuple, Optional, Tuple

__all__ = [
    "Attachment",
    "Block",
    "Change",
    "Details",
    "Diagnostic",
    "Document",
    "Entity",
    "Facts",
    "Field",
    "InternalError",
    "Item",
    "Marker",
    "Measurement",
    "Part",
    "Media",
    "MediaFact",
    "Node",
    "ParseError",
    "Projection",
    "Range",
    "Target",
    "Truncated",
    "parse",
    "parse_json",
    "project",
    "project_json",
    "serialize",
    "serialize_json",
    "targets",
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

__version__: str = _lib.gsx_version().decode()


class Range(NamedTuple):
    start: int
    end: int


@dataclass(frozen=True)
class Media:
    id: str
    cover: Optional[str] = None


@dataclass(frozen=True)
class Details:
    scheme: Optional[str] = None
    id: Optional[str] = None
    index: Optional[int] = None
    limit: Optional[str] = None
    maximum: Optional[int] = None
    code_point: Optional[int] = None


@dataclass(frozen=True)
class Node:
    type: str
    range: Range
    provenance: str = "authored"
    gap_before: Optional[int] = None
    children: List["Node"] = field(default_factory=list)
    depth: Optional[int] = None
    ordered: Optional[bool] = None
    start: Optional[int] = None
    spread: Optional[bool] = None
    checked: Optional[bool] = None
    lang: Optional[str] = None
    meta: Optional[str] = None
    value: Optional[str] = None
    align: Optional[List[Optional[str]]] = None
    url: Optional[str] = None
    source: Optional[str] = None
    title: Optional[str] = None
    datetime: Optional[str] = None
    unix: Optional[int] = None
    media_id: Optional[str] = None
    media_ids: Optional[List[str]] = None
    layout: Optional[str] = None


@dataclass(frozen=True)
class Marker:
    kind: str
    range: Range


@dataclass(frozen=True)
class Diagnostic:
    code: str
    severity: str
    range: Range
    details: Details


@dataclass(frozen=True)
class Document:
    children: List[Node]
    markers: List[Marker]
    diagnostics: List[Diagnostic]


class ParseError(Exception):
    def __init__(self, code: str, range: Optional[Range], details: Details) -> None:
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
    fields["children"] = [_node(child) for child in value.get("children", ())]
    return Node(**fields)


def _encode(text: str) -> bytes:
    try:
        return text.encode("utf-8")
    except UnicodeEncodeError:
        paired = text.encode("utf-16-le", "surrogatepass").decode("utf-16-le", "surrogatepass")
        return paired.encode("utf-8", "surrogatepass")


def _media(media: Iterable[Media]) -> ctypes.Array:
    entries = []
    for item in media:
        ident = item.id.encode("utf-8")
        cover = None if item.cover is None else item.cover.encode("utf-8")
        entries.append(_Media(ident, len(ident), cover, 0 if cover is None else len(cover)))
    return (_Media * len(entries))(*entries)


def _call(name: str, call) -> str:
    error = ctypes.c_void_p()
    output = call(ctypes.byref(error))
    if not output:
        message = _lib.gsx_error_message(error).decode("utf-8", "replace") if error else name + " failed"
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
        children=[_node(child) for child in document["children"]],
        markers=[Marker(item["kind"], Range(*item["range"])) for item in document["markers"]],
        diagnostics=[
            Diagnostic(item["code"], item["severity"], Range(*item["range"]), _details(item["details"]))
            for item in document["diagnostics"]
        ],
    )


@dataclass(frozen=True)
class MediaFact:
    """An attached media item, matched to media references by id."""

    id: str
    format: str
    alt: Optional[str] = None
    spoiler: bool = False
    inline: bool = False


@dataclass(frozen=True)
class Facts:
    media: Optional[str] = None
    media_count: int = 0
    title: Optional[str] = None
    max_characters: Optional[int] = None
    characters_reserved_per_url: Optional[int] = None
    caption_limit: Optional[int] = None
    entities_allowed: bool = False
    items: Optional[Tuple[MediaFact, ...]] = None
    target_url: Optional[str] = None
    description_limit: Optional[int] = None
    placement: Optional[str] = None
    hashtag_limit: Optional[int] = None
    """Instagram hashtags allowed in a caption; 5 without it."""
    spoiler_text: Optional[str] = None
    """Mastodon content warning: not rendered, but counted with the text against the status limit."""
    overflow: Optional[str] = None
    """What a body that does not fit becomes: "truncate" (the default) or "thread" where the target publishes threads."""


@dataclass(frozen=True)
class Entity:
    """A styled range of the field text in the target's offsets unit."""

    style: str
    range: Range
    value: Optional[str]


@dataclass(frozen=True)
class Field:
    name: str
    text: str
    links: List[Range]
    entities: List[Entity]


@dataclass(frozen=True)
class Measurement:
    field: str
    unit: str
    value: int
    maximum: Optional[int]
    block: Optional[int] = None
    """Per-item limits: the block and the item in it on blocks targets, else the index in items."""
    item: Optional[int] = None


@dataclass(frozen=True)
class Attachment:
    reference: str
    alt: Optional[str]


@dataclass(frozen=True)
class Item:
    reference: str
    alt: Optional[str]
    spoiler: bool


@dataclass(frozen=True)
class Block:
    """Text, media items with an optional layout, or a separator."""

    text: Optional[str] = None
    items: Optional[List[Item]] = None
    layout: Optional[str] = None
    separator: bool = False


@dataclass(frozen=True)
class Truncated:
    unit: str
    maximum: int
    value: int
    exact: bool


@dataclass(frozen=True)
class Change:
    class_: str
    feature: str
    reason: str
    code: Optional[str]
    field: str
    range: Optional[Range]
    truncated: Optional[Truncated]
    block: Optional[int] = None
    item: Optional[int] = None


@dataclass(frozen=True)
class Projection:
    fields: List[Field]
    measurements: List[Measurement]
    changes: List[Change]
    media: List[str]
    blocks: Optional[List[Block]] = None
    """Present for targets of the blocks/1 form only."""
    items: Optional[List[Attachment]] = None
    """Outside the blocks/1 form, one per media fact item with its fitted alt; absent without items."""
    parts: Optional[List["Part"]] = None
    """With overflow "thread", when the body did not fit: every post of the thread in order; the top level repeats the first."""


@dataclass(frozen=True)
class Part:
    fields: List[Field]
    measurements: List[Measurement]
    media: List[str]
    items: List[Attachment]


@dataclass(frozen=True)
class Target:
    name: str
    form: str
    offsets: Optional[str]
    digest: str
    manifest: str
    codes: List[str]
    """Every code a blocked change of this target can carry."""
    threads: bool
    """Whether overflow "thread" applies."""


class _MediaFact(ctypes.Structure):
    _fields_ = [
        ("id", ctypes.c_char_p),
        ("id_len", ctypes.c_size_t),
        ("format", ctypes.c_uint32),
        ("alt", ctypes.c_char_p),
        ("alt_len", ctypes.c_size_t),
        ("spoiler", ctypes.c_bool),
        ("inline", ctypes.c_bool),
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
        ("entities_allowed", ctypes.c_bool),
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
    title = None if facts.title is None else facts.title.encode("utf-8")
    target_url = None if facts.target_url is None else facts.target_url.encode("utf-8")
    spoiler_text = None if facts.spoiler_text is None else facts.spoiler_text.encode("utf-8")
    listed = () if facts.items is None else tuple(facts.items)
    items = None if facts.items is None else (_MediaFact * len(listed))()
    for index, item in enumerate(listed):
        if item.format not in _FORMATS:
            raise ValueError("facts.items format must be 'image', 'gif', 'video' or 'audio'")
        id_ = item.id.encode("utf-8")
        alt = None if item.alt is None else item.alt.encode("utf-8")
        items[index] = _MediaFact(
            id_, len(id_), _FORMATS[item.format], alt, 0 if alt is None else len(alt), bool(item.spoiler), bool(item.inline)
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
        bool(facts.entities_allowed),
        items,
        len(listed),
        _limit(facts.hashtag_limit, "facts.hashtag_limit"),
        spoiler_text,
        0 if spoiler_text is None else len(spoiler_text),
        _OVERFLOWS[facts.overflow],
    )


def project_json(text: str, target: str, facts: Facts = Facts(), media: Iterable[Media] = ()) -> str:
    data = _text(text)
    name = target.encode("utf-8")
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
        changes=[
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
        ],
        media=projection["media"],
        blocks=None if "blocks" not in projection else [_block(block) for block in projection["blocks"]],
        items=None if "items" not in projection else [Attachment(**item) for item in projection["items"]],
        parts=None if "parts" not in projection else [_part(part) for part in projection["parts"]],
    )


def _fields(fields: list) -> List[Field]:
    return [
        Field(
            item["name"],
            item["text"],
            [Range(*link) for link in item["links"]],
            [Entity(entity["style"], Range(*entity["range"]), entity["value"]) for entity in item["entities"]],
        )
        for item in fields
    ]


def _measurements(measurements: list) -> List[Measurement]:
    return [
        Measurement(item["field"], item["unit"], item["value"], item["maximum"], item.get("block"), item.get("item"))
        for item in measurements
    ]


def _part(part: dict) -> Part:
    return Part(
        _fields(part["fields"]),
        _measurements(part["measurements"]),
        part["media"],
        [Attachment(**item) for item in part["items"]],
    )


def _block(block: dict) -> Block:
    if "text" in block:
        return Block(text=block["text"])
    if "separator" in block:
        return Block(separator=True)
    return Block(items=[Item(**item) for item in block["items"]], layout=block["layout"])


def targets() -> List[Target]:
    listing = json.loads(_call("gsx_targets", _lib.gsx_targets))
    return [Target(**item) for item in listing["targets"]]

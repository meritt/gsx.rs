export type Range = readonly [start: number, end: number];

export interface Media {
  id: string;
  cover?: string | null;
}

interface NodeBase {
  range: Range;
  provenance?: 'literal';
  gapBefore?: number;
  children?: Node[];
}

export type Align = 'left' | 'right' | 'center' | null;

export type Node =
  | (NodeBase & {
      type:
        | 'paragraph'
        | 'thematicBreak'
        | 'blockquote'
        | 'tableRow'
        | 'tableCell'
        | 'emphasis'
        | 'strong'
        | 'delete'
        | 'underline'
        | 'highlight'
        | 'spoiler'
        | 'break';
    })
  | (NodeBase & { type: 'heading'; depth: number })
  | (NodeBase & { type: 'list'; ordered: boolean; start: number | null; spread: boolean })
  | (NodeBase & { type: 'listItem'; spread: boolean; checked: boolean | null })
  | (NodeBase & { type: 'code'; lang: string | null; meta: string | null; value: string })
  | (NodeBase & { type: 'table'; align: Align[] })
  | (NodeBase & { type: 'text' | 'inlineCode'; value: string })
  | (NodeBase & { type: 'link'; url: string; source: string; title: string | null })
  | (NodeBase & { type: 'datetime'; datetime: string | null; unix: number })
  | (NodeBase & { type: 'mediaReference'; mediaId: string })
  | (NodeBase & { type: 'mediaGroup'; mediaIds: string[]; layout: Layout });

export type Layout = 'collage' | 'slideshow';

export type MarkerKind =
  | 'emphasis'
  | 'strong'
  | 'strikethrough'
  | 'spoiler'
  | 'underline'
  | 'highlight'
  | 'codeText'
  | 'codeFence'
  | 'heading'
  | 'thematicBreak'
  | 'blockQuote'
  | 'listItem'
  | 'taskCheck'
  | 'tableDelimiter'
  | 'link'
  | 'autolink'
  | 'escape'
  | 'characterReference'
  | 'hardBreak'
  | 'directive'
  | 'media';

export interface Marker {
  kind: MarkerKind;
  range: Range;
}

export type Code =
  | 'DATETIME_INVALID'
  | 'DATETIME_TIMEZONE_REQUIRED'
  | 'LINK_LITERALIZED'
  | 'MARKDOWN_INVALID_TEXT'
  | 'MARKDOWN_MALFORMED'
  | 'MARKDOWN_TOO_COMPLEX'
  | 'MEDIA_DIRECTIVE_INVALID'
  | 'MEDIA_REFERENCE_CONTEXT_UNSUPPORTED'
  | 'MEDIA_REFERENCE_COVER'
  | 'MEDIA_REFERENCE_DUPLICATE'
  | 'MEDIA_REFERENCE_UNKNOWN';

export type Details =
  | Record<string, never>
  | { scheme: 'http' | 'https' | 'relative' | 'unsupported' }
  | { id: string; index?: number }
  | { limit: string; maximum: number }
  | { codePoint: number };

export interface Diagnostic {
  code: Code;
  severity: 'warning' | 'error';
  range: Range;
  details: Details;
}

export interface Document {
  children: Node[];
  markers: Marker[];
  diagnostics: Diagnostic[];
}

export interface WireError {
  code: Code;
  range: Range | null;
  details: Details;
}

export type Wire = { document: Document } | { error: WireError };

export interface Facts {
  media?: 'image' | 'video' | 'gif' | 'audio' | null;
  mediaCount?: number;
  title?: string | null;
  /** Appended after the body, or replacing a blank body, on targets that declare `target_url`; never cut. */
  targetUrl?: string | null;
  maxCharacters?: number | null;
  charactersReservedPerUrl?: number | null;
  captionLimit?: number | null;
  /** Mastodon instance media description limit in code points; without it alt is not fitted. */
  descriptionLimit?: number | null;
  entitiesAllowed?: boolean;
  items?: Iterable<MediaFact>;
}

/** An attached media item, matched to media references by `id`. */
export interface MediaFact {
  id: string;
  format: 'image' | 'gif' | 'video' | 'audio';
  alt?: string | null;
  spoiler?: boolean;
  inline?: boolean;
}

export type FieldName = 'text' | 'title' | 'description' | 'alt';

export type Style = 'bold' | 'italic' | 'underline' | 'strikethrough' | 'spoiler' | 'code' | 'link' | 'tag';

export type Unit =
  | 'graphemes'
  | 'code-points'
  | 'utf16-code-units'
  | 'utf8-bytes'
  | 'x-weighted'
  | 'mastodon-weighted'
  | 'threads-weighted'
  | 'hashtags'
  | 'mentions';

export type Class = 'approximated' | 'textual_fallback' | 'unwrapped' | 'omitted' | 'blocked';

export type Feature =
  | 'heading'
  | 'thematic_break'
  | 'blockquote'
  | 'bullet_list'
  | 'ordered_list'
  | 'compound_list'
  | 'code_block'
  | 'table'
  | 'task_list'
  | 'media'
  | 'emphasis'
  | 'strong'
  | 'strikethrough'
  | 'underline'
  | 'highlight'
  | 'spoiler'
  | 'inline_code'
  | 'link'
  | 'link_title'
  | 'datetime'
  | 'angle_brackets'
  | 'text'
  | 'title'
  | 'target_url'
  | 'hashtag'
  | 'mention';

/** Entity ranges are in the target's `offsets` unit; links are UTF-16 ranges of the field text. */
export interface Entity {
  style: Style;
  range: Range;
  value: string | null;
}

export interface Field {
  name: FieldName;
  text: string;
  links: Range[];
  entities: Entity[];
}

export interface Measurement {
  field: FieldName;
  unit: Unit;
  value: number;
  maximum: number | null;
  /** Per-item limits: the block and the item in it on blocks targets, else the index in `items`. */
  block?: number;
  item?: number;
}

export interface Item {
  reference: string;
  alt: string | null;
  spoiler: boolean;
}

export type Block = { text: string } | { items: Item[]; layout: Layout | null };

export interface Truncated {
  unit: Unit;
  maximum: number;
  value: number;
  exact: boolean;
}

export interface Change {
  class: Class;
  feature: Feature;
  reason: string;
  /** The reason in upper case when the change is blocked. */
  code: string | null;
  field: FieldName;
  range: Range | null;
  truncated: Truncated | null;
  /** Alt changes: the block and the item in it on blocks targets, else the index in `items`. */
  block?: number;
  item?: number;
}

export interface Projection {
  fields: Field[];
  measurements: Measurement[];
  changes: Change[];
  media: string[];
  /** Present for targets of the `blocks/1` form only. */
  blocks?: Block[];
  /** Outside the `blocks/1` form, one per media fact item with its fitted alt; absent without items. */
  items?: Attachment[];
}

export interface Attachment {
  reference: string;
  alt: string | null;
}

export type ProjectionWire = { projection: Projection } | { error: WireError };

export type SerializedWire = { serialized: string } | { error: WireError };

export interface Target {
  name: string;
  form: 'plain/1' | 'entities/1' | 'html/1' | 'markdown/1' | 'blocks/1';
  offsets: 'utf8' | 'utf16' | null;
  digest: string;
  manifest: string;
}

export declare class ParseError extends Error {
  readonly code: Code;
  readonly range: Range | null;
  readonly details: Details;
}

export declare class VersionError extends Error {
  readonly expected: string;
  readonly found: string | undefined;
}

export interface GSX {
  readonly VERSION: string;
  readonly ParseError: typeof ParseError;
  readonly VersionError: typeof VersionError;
  parse(text: string, media?: Iterable<Media>): Document;
  parseJSON(text: string, media?: Iterable<Media>): string;
  serialize(text: string, media?: Iterable<Media>): string;
  serializeJSON(text: string, media?: Iterable<Media>): string;
  project(text: string, target: string, facts?: Facts, media?: Iterable<Media>): Projection;
  projectJSON(text: string, target: string, facts?: Facts, media?: Iterable<Media>): string;
  targets(): readonly Readonly<Target>[];
}

export declare function load(
  source?: URL | string | Response | Promise<Response> | WebAssembly.Module | BufferSource
): Promise<GSX>;

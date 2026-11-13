export const load = (() => {
  const VERSION = '@VERSION@';

  class ParseError extends Error {
    constructor({ code, range, details }) {
      super(range === null ? code : `${code} at ${range[0]}..${range[1]}`);
      this.name = 'ParseError';
      this.code = code;
      this.range = range;
      this.details = details;
    }
  }

  class VersionError extends Error {
    constructor(found) {
      super(
        `gsx.wasm is ${found === undefined ? 'from another build' : `version ${found}`}, gsx.js is version ${VERSION}`
      );
      this.name = 'VersionError';
      this.expected = VERSION;
      this.found = found;
    }
  }

  function firstLoneSurrogate(text) {
    for (let i = 0; i < text.length; i++) {
      const unit = text.charCodeAt(i);

      if (unit >= 0xd800 && unit <= 0xdbff) {
        const next = text.charCodeAt(i + 1);

        if (next >= 0xdc00 && next <= 0xdfff) {
          i++;
          continue;
        }

        return i;
      }

      if (unit >= 0xdc00 && unit <= 0xdfff) {
        return i;
      }
    }

    return text.length;
  }

  function invalid(text) {
    if (typeof text !== 'string') {
      throw new TypeError('text must be a string');
    }

    if (text.isWellFormed()) return null;

    const at = firstLoneSurrogate(text);
    return [at, text.charCodeAt(at)];
  }

  function split(media) {
    const ids = [];
    const covers = [];

    for (const { id, cover } of media) {
      if (typeof id !== 'string' || !id.isWellFormed()) {
        throw new TypeError('media id must be a well-formed string');
      }

      if (cover != null && (typeof cover !== 'string' || !cover.isWellFormed())) {
        throw new TypeError('media cover must be a well-formed string, null or undefined');
      }

      ids.push(id);
      covers.push(cover ?? undefined);
    }

    return [ids, covers];
  }

  const KINDS = { __proto__: null, image: 1, video: 2, gif: 3, audio: 4 };
  const PLACEMENTS = { __proto__: null, post: 1, reel: 2, story: 3 };
  const OVERFLOWS = { __proto__: null, truncate: 0, thread: 1 };
  const FORMATS = { image: 0, gif: 1, video: 2, audio: 3 };

  function mediaFacts(items) {
    const ids = [];
    const formats = [];
    const alts = [];
    const flags = [];

    for (const { id, format, alt = null, spoiler = false, inline = false } of items) {
      if (typeof id !== 'string' || !id.isWellFormed()) {
        throw new TypeError('facts.items id must be a well-formed string');
      }

      if (!Object.hasOwn(FORMATS, format)) {
        throw new TypeError("facts.items format must be 'image', 'gif', 'video' or 'audio'");
      }

      if (alt !== null && (typeof alt !== 'string' || !alt.isWellFormed())) {
        throw new TypeError('facts.items alt must be a well-formed string or null');
      }

      if (typeof spoiler !== 'boolean' || typeof inline !== 'boolean') {
        throw new TypeError('facts.items spoiler and inline must be booleans');
      }

      ids.push(id);
      formats.push(FORMATS[format]);
      alts.push(alt ?? undefined);
      flags.push(Number(spoiler) | (Number(inline) << 1));
    }

    return [ids, Uint8Array.from(formats), alts, Uint8Array.from(flags)];
  }

  function count(value, name) {
    if (value == null) return undefined;

    if (!Number.isInteger(value) || value < 0 || value > 0xffffffff) {
      throw new RangeError(`${name} must be an integer from 0 to 2^32 - 1`);
    }

    return value;
  }

  function unwrap(json, key) {
    const result = JSON.parse(json);

    if (result.error) {
      throw new ParseError(result.error);
    }

    return result[key];
  }

  function bind(instance, instantiate) {
    const encoder = new TextEncoder();
    let { wasm, memory } = instance;
    let bytes = new Uint8Array(0);

    function call(run) {
      try {
        return run();
      } catch (error) {
        if (error instanceof WebAssembly.RuntimeError) {
          ({ wasm, memory } = instantiate());
          bytes = new Uint8Array(0);
        }

        throw error;
      }
    }

    function pass(text) {
      const capacity = text.length * 3;
      const at = wasm.input(capacity);
      if (bytes.buffer !== memory.buffer) bytes = new Uint8Array(memory.buffer);
      return encoder.encodeInto(text, bytes.subarray(at, at + capacity)).written;
    }

    function parseJSON(text, media = []) {
      const entries = split(media);
      const at = invalid(text);

      return call(() => (at ? wasm.invalidText(...at) : wasm.parse(pass(text), ...entries)));
    }

    function serializeJSON(text, media = []) {
      const entries = split(media);
      const at = invalid(text);

      return call(() => (at ? wasm.invalidText(...at) : wasm.serialize(pass(text), ...entries)));
    }

    function projectJSON(text, target, facts = {}, media = []) {
      if (typeof target !== 'string') {
        throw new TypeError('target must be a string');
      }

      const { title = null, targetUrl = null, spoilerText = null, entitiesAllowed = false, items = null } = facts;
      const kind = facts.media == null ? 0 : KINDS[facts.media];

      if (kind === undefined) {
        throw new TypeError("facts.media must be 'image', 'video', 'gif', 'audio', null or undefined");
      }

      const placement = facts.placement == null ? 0 : PLACEMENTS[facts.placement];

      if (placement === undefined) {
        throw new TypeError("facts.placement must be 'post', 'reel', 'story', null or undefined");
      }

      const overflow = facts.overflow == null ? 0 : OVERFLOWS[facts.overflow];

      if (overflow === undefined) {
        throw new TypeError("facts.overflow must be 'truncate', 'thread', null or undefined");
      }

      if (title !== null && (typeof title !== 'string' || !title.isWellFormed())) {
        throw new TypeError('facts.title must be a well-formed string or null');
      }

      if (targetUrl !== null && (typeof targetUrl !== 'string' || !targetUrl.isWellFormed())) {
        throw new TypeError('facts.targetUrl must be a well-formed string or null');
      }

      if (spoilerText !== null && (typeof spoilerText !== 'string' || !spoilerText.isWellFormed())) {
        throw new TypeError('facts.spoilerText must be a well-formed string or null');
      }

      if (typeof entitiesAllowed !== 'boolean') {
        throw new TypeError('facts.entitiesAllowed must be a boolean');
      }

      const entries = split(media);
      const described = mediaFacts(items ?? []);
      const limits = [
        count(facts.mediaCount, 'facts.mediaCount') ?? 0,
        placement,
        title ?? undefined,
        targetUrl ?? undefined,
        count(facts.maxCharacters, 'facts.maxCharacters'),
        count(facts.charactersReservedPerUrl, 'facts.charactersReservedPerUrl'),
        count(facts.captionLimit, 'facts.captionLimit'),
        count(facts.descriptionLimit, 'facts.descriptionLimit'),
        count(facts.hashtagLimit, 'facts.hashtagLimit'),
        spoilerText ?? undefined,
        overflow,
      ];
      const at = invalid(text);

      return call(() =>
        at
          ? wasm.invalidText(...at)
          : wasm.project(pass(text), ...entries, target, kind, ...limits, entitiesAllowed, items !== null, ...described)
      );
    }

    function isMediaId(id) {
      if (typeof id !== 'string') {
        throw new TypeError('id must be a string');
      }

      return call(() => wasm.isMediaId(pass(id)));
    }

    let listed;

    return {
      parse: (text, media) => unwrap(parseJSON(text, media), 'document'),
      parseJSON,
      serialize: (text, media) => unwrap(serializeJSON(text, media), 'serialized'),
      serializeJSON,
      project: (text, target, facts, media) => unwrap(projectJSON(text, target, facts, media), 'projection'),
      projectJSON,
      targets: () => (listed ??= Object.freeze(JSON.parse(call(() => wasm.targets())).targets.map(Object.freeze))),
      isMediaId,
    };
  }

  function glue() {
    /* @glue */
    return wasm_bindgen;
  }

  async function compile(source) {
    if (source instanceof WebAssembly.Module) return source;
    if (typeof source === 'string') source = new URL(source, import.meta.url);
    if (source instanceof URL) {
      if (source.protocol === 'file:') {
        const { readFile } = await import('node:fs/promises');
        return WebAssembly.compile(await readFile(source));
      }
      source = fetch(source);
    }
    if (source instanceof Promise || (typeof Response === 'function' && source instanceof Response)) {
      return WebAssembly.compileStreaming(source);
    }
    return WebAssembly.compile(source);
  }

  return async function load(source = new URL('./gsx.wasm', import.meta.url)) {
    const module = await compile(source);

    if (!WebAssembly.Module.exports(module).some(({ name }) => name === 'version')) {
      throw new VersionError(undefined);
    }

    function instantiate() {
      const wasm = glue();
      return { wasm, memory: wasm.initSync({ module }).memory };
    }

    let instance;

    try {
      instance = instantiate();
    } catch (error) {
      throw error instanceof WebAssembly.LinkError ? new VersionError(undefined) : error;
    }

    const found = instance.wasm.version();

    if (found !== VERSION) {
      throw new VersionError(found);
    }

    const { wasm } = instance;

    return Object.freeze({
      ...bind(instance, instantiate),
      VERSION,
      MEDIA_ID_MAX_BYTES: wasm.mediaIdMaxBytes(),
      MEDIA_ID_PATTERN: wasm.mediaIdPattern(),
      MEDIA_GROUP_MIN: wasm.mediaGroupMin(),
      MEDIA_GROUP_MAX: wasm.mediaGroupMax(),
      ParseError,
      VersionError,
    });
  };
})();

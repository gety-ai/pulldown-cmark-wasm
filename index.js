// Loader for pulldown_cmark_wasm.wasm. Works in browsers, Web Workers, Node and Bun.

export const NONE = 0xffffffff;
export const ENTRY_SIZE = 4;
export const VERBATIM = 1;

/** pulldown-cmark `Options` bits, plus FORCE_STRINGS (always store payloads). */
export const Options = Object.freeze({
  TABLES: 1 << 1,
  FOOTNOTES: 1 << 2,
  STRIKETHROUGH: 1 << 3,
  TASKLISTS: 1 << 4,
  SMART_PUNCTUATION: 1 << 5,
  HEADING_ATTRIBUTES: 1 << 6,
  YAML_STYLE_METADATA_BLOCKS: 1 << 7,
  PLUSES_DELIMITED_METADATA_BLOCKS: 1 << 8,
  OLD_FOOTNOTES: (1 << 9) | (1 << 2),
  MATH: 1 << 10,
  GFM: 1 << 11,
  DEFINITION_LIST: 1 << 12,
  SUPERSCRIPT: 1 << 13,
  SUBSCRIPT: 1 << 14,
  WIKILINKS: 1 << 15,
  CONTAINER_EXTENSIONS: 1 << 16,
  HIGHLIGHT: 1 << 17,
  CJK_FRIENDLY_EMPHASIS: 1 << 18,
  FORCE_STRINGS: 1 << 31,
});

export const Op = Object.freeze({
  START: 1,
  END: 2,
  TEXT: 3,
  CODE: 4,
  HTML: 5,
  INLINE_HTML: 6,
  SOFT_BREAK: 7,
  HARD_BREAK: 8,
  RULE: 9,
  TASK_LIST_MARKER: 10,
  FOOTNOTE_REFERENCE: 11,
  INLINE_MATH: 12,
  DISPLAY_MATH: 13,
});

export const Tag = Object.freeze({
  PARAGRAPH: 1,
  HEADING: 2,
  BLOCK_QUOTE: 3,
  CODE_BLOCK: 4,
  HTML_BLOCK: 5,
  LIST: 6,
  ITEM: 7,
  FOOTNOTE_DEFINITION: 8,
  TABLE: 9,
  TABLE_HEAD: 10,
  TABLE_ROW: 11,
  TABLE_CELL: 12,
  EMPHASIS: 13,
  STRONG: 14,
  STRIKETHROUGH: 15,
  LINK: 16,
  IMAGE: 17,
  METADATA_BLOCK: 18,
  OTHER: 31,
});

export const op = (entries, i) => entries[i] >>> 24;
export const tag = (entries, i) => (entries[i] >>> 16) & 0xff;
export const small = (entries, i) => entries[i] & 0xffff;

const decoder = new TextDecoder();

/** Decodes string `index` of a parse result. */
export function readString(result, index) {
  const start = index === 0 ? 0 : result.stringEnds[index - 1];
  return decoder.decode(result.strings.subarray(start, result.stringEnds[index]));
}

/**
 * Instantiates the module. `source` may be wasm bytes, a compiled
 * WebAssembly.Module, a fetch Response, or a promise of any of these.
 */
export async function createParser(source) {
  const input = await source;
  let instance;
  if (input instanceof WebAssembly.Module) {
    instance = await WebAssembly.instantiate(input, {});
  } else {
    const bytes = typeof input?.arrayBuffer === 'function' ? await input.arrayBuffer() : input;
    ({ instance } = await WebAssembly.instantiate(bytes, {}));
  }
  const ex = instance.exports;
  const encoder = new TextEncoder();
  return function parse(markdown, options = 0) {
    // UTF-16 -> UTF-8 needs at most 3 bytes per code unit.
    const capacity = Math.max(markdown.length * 3, 1);
    const input = ex.md_alloc(capacity);
    let header;
    try {
      const { written } = encoder.encodeInto(markdown, new Uint8Array(ex.memory.buffer, input, capacity));
      header = ex.md_parse(input, written, options >>> 0);
    } finally {
      ex.md_free(input, capacity);
    }
    // Memory may grow during parsing, so views are created afterwards.
    const memory = ex.memory.buffer;
    const [entriesPtr, entriesLen, stringsPtr, stringsLen, endsPtr, endsLen] = new Uint32Array(memory, header, 6);
    const result = {
      entries: new Uint32Array(memory, entriesPtr, entriesLen).slice(),
      strings: new Uint8Array(memory, stringsPtr, stringsLen).slice(),
      stringEnds: new Uint32Array(memory, endsPtr, endsLen).slice(),
    };
    ex.md_free_result(header);
    return result;
  };
}

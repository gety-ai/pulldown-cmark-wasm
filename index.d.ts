/** `aux` value meaning "no string". */
export declare const NONE: 0xffffffff;
/** Number of u32 words per event in `ParseResult.entries`. */
export declare const ENTRY_SIZE: 4;
/** `small` flag on TEXT / HTML / INLINE_HTML: the payload equals the source slice. */
export declare const VERBATIM: 1;

export declare const Options: Readonly<{
  TABLES: number;
  FOOTNOTES: number;
  STRIKETHROUGH: number;
  TASKLISTS: number;
  SMART_PUNCTUATION: number;
  HEADING_ATTRIBUTES: number;
  YAML_STYLE_METADATA_BLOCKS: number;
  PLUSES_DELIMITED_METADATA_BLOCKS: number;
  OLD_FOOTNOTES: number;
  MATH: number;
  GFM: number;
  DEFINITION_LIST: number;
  SUPERSCRIPT: number;
  SUBSCRIPT: number;
  WIKILINKS: number;
  CONTAINER_EXTENSIONS: number;
  HIGHLIGHT: number;
  CJK_FRIENDLY_EMPHASIS: number;
  /** Store every text-like payload even when it equals the source slice. */
  FORCE_STRINGS: number;
}>;

export declare const Op: Readonly<{
  START: 1;
  END: 2;
  TEXT: 3;
  CODE: 4;
  HTML: 5;
  INLINE_HTML: 6;
  SOFT_BREAK: 7;
  HARD_BREAK: 8;
  RULE: 9;
  TASK_LIST_MARKER: 10;
  FOOTNOTE_REFERENCE: 11;
  INLINE_MATH: 12;
  DISPLAY_MATH: 13;
}>;

export declare const Tag: Readonly<{
  PARAGRAPH: 1;
  HEADING: 2;
  BLOCK_QUOTE: 3;
  CODE_BLOCK: 4;
  HTML_BLOCK: 5;
  LIST: 6;
  ITEM: 7;
  FOOTNOTE_DEFINITION: 8;
  TABLE: 9;
  TABLE_HEAD: 10;
  TABLE_ROW: 11;
  TABLE_CELL: 12;
  EMPHASIS: 13;
  STRONG: 14;
  STRIKETHROUGH: 15;
  LINK: 16;
  IMAGE: 17;
  METADATA_BLOCK: 18;
  /** Any tag without a dedicated id. */
  OTHER: 31;
}>;

/**
 * Plain typed arrays (structured-cloneable and transferable).
 * Event `i` occupies `entries[i * 4 .. i * 4 + 4]`, see README.md.
 */
export interface ParseResult {
  entries: Uint32Array;
  /** Concatenated UTF-8 strings. */
  strings: Uint8Array;
  /** End offset of each string in `strings`. */
  stringEnds: Uint32Array;
}

/** Op of the event whose first word is at `entries[i]`. */
export declare function op(entries: Uint32Array, i: number): number;
/** Tag of the event whose first word is at `entries[i]`. */
export declare function tag(entries: Uint32Array, i: number): number;
/** Small field of the event whose first word is at `entries[i]`. */
export declare function small(entries: Uint32Array, i: number): number;

export declare function readString(result: ParseResult, index: number): string;

export type ParserSource =
  | BufferSource
  | WebAssembly.Module
  | Response
  | PromiseLike<BufferSource | WebAssembly.Module | Response>;

/** Ranges in the result are UTF-16 offsets into `markdown`. */
export type Parse = (markdown: string, options?: number) => ParseResult;

export declare function createParser(source: ParserSource): Promise<Parse>;

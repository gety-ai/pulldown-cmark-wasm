# pulldown-cmark-wasm

[pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) compiled to
`wasm32-unknown-unknown`. It parses Markdown into a flat event table whose
source ranges are **UTF-16 offsets** into the input string, so JavaScript can
map every event back to `markdown.slice(start, end)` without conversion.

- No wasm-bindgen, threads, SharedArrayBuffer, COOP/COEP or exception-handling
  proposal: runs in any browser engine, Web Workers, Node and Bun.
- ~260 KB wasm (~100 KB gzip). A 10 MB document parses in ~70 ms.
- Built from pulldown-cmark git `953e44a`, which includes CJK-friendly emphasis
  (not on crates.io yet).

## Usage

```js
import { createParser, Options, Op, Tag, ENTRY_SIZE, NONE, VERBATIM, readString } from 'pulldown-cmark-wasm';
import wasmUrl from 'pulldown-cmark-wasm/pulldown_cmark_wasm.wasm?url'; // Vite

const parse = await createParser(fetch(wasmUrl));
const result = parse(markdown, Options.TABLES | Options.STRIKETHROUGH | Options.CJK_FRIENDLY_EMPHASIS);
```

`createParser` accepts wasm bytes, a `WebAssembly.Module`, a `Response`, or a
promise of any of these. The result holds three plain typed arrays and can be
transferred from a Worker as is.

## Event table

`result.entries` is a `Uint32Array` with `ENTRY_SIZE` (4) words per event, in
document order:

| Word | Content |
|---|---|
| 0 | `op << 24 \| tag << 16 \| small` |
| 1 | UTF-16 start offset |
| 2 | UTF-16 end offset |
| 3 | `aux`: string index, number, or `NONE` |

Strings: `readString(result, index)` decodes string `index` from
`result.strings` (UTF-8 bytes) and `result.stringEnds` (end offsets).

| Op | Meaning | `small` | `aux` |
|---|---|---|---|
| `START` / `END` | open / close `tag` | see below | see below |
| `TEXT`, `HTML`, `INLINE_HTML` | text, block HTML line, inline HTML | `VERBATIM` if the payload equals the source slice | payload string, or `NONE` when verbatim (slice the source instead) |
| `CODE` | inline code (range includes backticks) | 0 | code string |
| `INLINE_MATH`, `DISPLAY_MATH` | math (range includes delimiters) | 0 | math string |
| `FOOTNOTE_REFERENCE` | `[^label]` | 0 | label string |
| `SOFT_BREAK`, `HARD_BREAK`, `RULE` | | 0 | `NONE` |
| `TASK_LIST_MARKER` | `[ ]` / `[x]` | 0 | 1 if checked |

`START` attributes:

| Tag | `small` | `aux` |
|---|---|---|
| `HEADING` | level 1–6 | `NONE` |
| `BLOCK_QUOTE` | GFM alert kind: 0 none, 1 note, 2 tip, 3 important, 4 warning, 5 caution | `NONE` |
| `CODE_BLOCK` | 1 fenced, 0 indented | info string (fenced only) |
| `LIST` | 0 | 0 for bullet lists, `start + 1` for ordered lists |
| `TABLE` | 0 | alignment string, one char per column: `n` `l` `c` `r` |
| `LINK`, `IMAGE` | 0 | destination string; the title is string `aux + 1` |
| `FOOTNOTE_DEFINITION` | 0 | label string |
| `OTHER` | 0 | `NONE` (any tag without a dedicated id) |

Text with entities or escapes is split by pulldown-cmark: `a &amp; \*b` yields
`a ` (verbatim), `&` (not verbatim, source `&amp;`) and `*b` (verbatim, the
backslash is excluded from the range).

`Options.FORCE_STRINGS` stores every payload even when verbatim; it exists for
verifying ranges.

This table is the package contract. Any change to it is a major version bump.

## Build

```sh
sh build.sh      # writes dist/pulldown_cmark_wasm.wasm (committed)
npm test         # cargo test + Node tests against dist/
```

The toolchain is pinned in `rust-toolchain.toml`. Paths are remapped, so the
output is byte-identical regardless of the checkout location.

## License

MIT. Bundled crates are listed in `THIRD_PARTY_LICENSES`.

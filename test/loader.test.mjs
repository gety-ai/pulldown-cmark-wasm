import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { createParser, ENTRY_SIZE, NONE, Op, Options, Tag, VERBATIM, op, readString, small, tag } from '../index.js';

const wasm = await readFile(new URL('../dist/pulldown_cmark_wasm.wasm', import.meta.url));
const parse = await createParser(wasm);
const GFM_CJK = Options.TABLES | Options.FOOTNOTES | Options.STRIKETHROUGH | Options.TASKLISTS | Options.CJK_FRIENDLY_EMPHASIS;

const events = (result) => {
  const out = [];
  for (let i = 0; i < result.entries.length; i += ENTRY_SIZE) {
    const e = result.entries;
    out.push({ op: op(e, i), tag: tag(e, i), small: small(e, i), start: e[i + 1], end: e[i + 2], aux: e[i + 3] });
  }
  return out;
};

test('verbatim ranges are UTF-16 offsets into the JS string (astral chars, CJK, entities)', () => {
  const src = '# 标题 😀\n\n- 中文 **粗体** [链接](https://e.com)\n- `code` &amp; \\* 😀😀 tail\n\n| a | 表 |\n|-|-|\n| 1 | 2 |\n'.repeat(50);
  const result = parse(src, GFM_CJK | Options.FORCE_STRINGS);
  let verbatim = 0;
  for (const e of events(result)) {
    assert.ok(e.start <= e.end && e.end <= src.length);
    if (e.op === Op.TEXT && e.small === VERBATIM) {
      verbatim++;
      assert.equal(src.slice(e.start, e.end), readString(result, e.aux));
    }
  }
  assert.ok(verbatim > 500);
});

test('verbatim text stores no string unless FORCE_STRINGS is set', () => {
  const result = parse('a &amp; b', GFM_CJK);
  const texts = events(result).filter((e) => e.op === Op.TEXT);
  assert.deepEqual(
    texts.map((e) => (e.aux === NONE ? null : readString(result, e.aux))),
    [null, '&', null],
  );
});

test('empty input and lone surrogates are accepted', () => {
  assert.equal(parse('', GFM_CJK).entries.length, 0);
  const src = 'a\uD800b';
  const text = events(parse(src, GFM_CJK)).find((e) => e.op === Op.TEXT);
  assert.deepEqual([text.start, text.end], [0, src.length]);
});

test('results are plain transferable typed arrays', () => {
  const result = parse('**x**', GFM_CJK);
  const cloned = structuredClone(result, { transfer: [result.entries.buffer, result.strings.buffer, result.stringEnds.buffer] });
  assert.ok(events(cloned).some((e) => e.op === Op.START && e.tag === Tag.STRONG));
});

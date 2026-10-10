//! pulldown-cmark compiled to `wasm32-unknown-unknown` with a raw C ABI.
//!
//! The input is UTF-8 Markdown; the output is a flat event table whose source
//! ranges are UTF-16 offsets, so JavaScript can slice the original string
//! directly. The table layout is documented in README.md and is the package
//! contract: any change to it is a breaking change.
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, Options, Parser, Tag, TagEnd,
};

/// `aux` value meaning "no string".
pub const NONE: u32 = u32::MAX;
/// Option bit (not a pulldown-cmark flag) that stores every text-like payload,
/// even when it equals the source slice. Used to verify ranges.
pub const FORCE_STRINGS: u32 = 1 << 31;

/// Small-field flag on Text / Html / InlineHtml: payload equals the source slice.
pub const VERBATIM: u32 = 1;

pub mod op {
    pub const START: u32 = 1;
    pub const END: u32 = 2;
    pub const TEXT: u32 = 3;
    pub const CODE: u32 = 4;
    pub const HTML: u32 = 5;
    pub const INLINE_HTML: u32 = 6;
    pub const SOFT_BREAK: u32 = 7;
    pub const HARD_BREAK: u32 = 8;
    pub const RULE: u32 = 9;
    pub const TASK_LIST_MARKER: u32 = 10;
    pub const FOOTNOTE_REFERENCE: u32 = 11;
    pub const INLINE_MATH: u32 = 12;
    pub const DISPLAY_MATH: u32 = 13;
}

pub mod tag {
    pub const PARAGRAPH: u32 = 1;
    pub const HEADING: u32 = 2;
    pub const BLOCK_QUOTE: u32 = 3;
    pub const CODE_BLOCK: u32 = 4;
    pub const HTML_BLOCK: u32 = 5;
    pub const LIST: u32 = 6;
    pub const ITEM: u32 = 7;
    pub const FOOTNOTE_DEFINITION: u32 = 8;
    pub const TABLE: u32 = 9;
    pub const TABLE_HEAD: u32 = 10;
    pub const TABLE_ROW: u32 = 11;
    pub const TABLE_CELL: u32 = 12;
    pub const EMPHASIS: u32 = 13;
    pub const STRONG: u32 = 14;
    pub const STRIKETHROUGH: u32 = 15;
    pub const LINK: u32 = 16;
    pub const IMAGE: u32 = 17;
    pub const METADATA_BLOCK: u32 = 18;
    /// Any tag without a dedicated id (definition lists, superscript, ...).
    pub const OTHER: u32 = 31;
}

/// Checkpoint spacing of the UTF-8 -> UTF-16 offset index, as a shift.
const CHECKPOINT_SHIFT: usize = 6;

/// Checkpoint flag: the 64-byte chunk is pure ASCII, so offsets inside it map
/// 1:1 and `get` needs no scan. Inputs stay far below 2^31 UTF-16 units.
const ASCII_CHUNK: u32 = 1 << 31;

/// UTF-8 byte offset -> UTF-16 offset, via a checkpoint every 64 bytes.
struct Utf16Index<'a> {
    bytes: &'a [u8],
    /// UTF-16 offset of each chunk start, tagged with `ASCII_CHUNK`.
    checkpoints: Vec<u32>,
}

impl<'a> Utf16Index<'a> {
    fn new(src: &'a str) -> Self {
        let bytes = src.as_bytes();
        let mut checkpoints = Vec::with_capacity((bytes.len() >> CHECKPOINT_SHIFT) + 1);
        let mut units = 0u32;
        for chunk in bytes.chunks(1 << CHECKPOINT_SHIFT) {
            if chunk.is_ascii() {
                checkpoints.push(units | ASCII_CHUNK);
                units += chunk.len() as u32;
            } else {
                checkpoints.push(units);
                units += chunk.iter().map(|&b| utf16_units(b)).sum::<u32>();
            }
        }
        checkpoints.push(units);
        Self { bytes, checkpoints }
    }

    /// `byte` must be a char boundary.
    fn get(&self, byte: usize) -> u32 {
        let block = byte >> CHECKPOINT_SHIFT;
        let block_start = block << CHECKPOINT_SHIFT;
        let checkpoint = self.checkpoints[block];
        if checkpoint & ASCII_CHUNK != 0 {
            return (checkpoint & !ASCII_CHUNK) + (byte - block_start) as u32;
        }
        let tail = &self.bytes[block_start..byte];
        checkpoint + tail.iter().map(|&b| utf16_units(b)).sum::<u32>()
    }
}

/// UTF-16 code units of the character whose UTF-8 encoding contains `b`,
/// counted on the leading byte only.
#[inline]
fn utf16_units(b: u8) -> u32 {
    match b {
        0x80..=0xBF => 0,
        0xF0..=0xFF => 2,
        _ => 1,
    }
}

/// Parse result: 4 `u32` per event plus a string table.
#[derive(Default)]
pub struct Output {
    pub entries: Vec<u32>,
    /// Concatenated UTF-8 strings.
    pub strings: Vec<u8>,
    /// End offset of each string in `strings`.
    pub string_ends: Vec<u32>,
}

impl Output {
    fn push_str(&mut self, s: &str) -> u32 {
        self.strings.extend_from_slice(s.as_bytes());
        self.string_ends.push(self.strings.len() as u32);
        (self.string_ends.len() - 1) as u32
    }

    fn push(&mut self, op: u32, tag: u32, small: u32, start: u32, end: u32, aux: u32) {
        self.entries
            .extend_from_slice(&[op << 24 | tag << 16 | small, start, end, aux]);
    }

    pub fn string(&self, index: u32) -> &str {
        let index = index as usize;
        let start = if index == 0 { 0 } else { self.string_ends[index - 1] as usize };
        std::str::from_utf8(&self.strings[start..self.string_ends[index] as usize]).unwrap()
    }
}

fn tag_id(tag: &Tag) -> u32 {
    match tag {
        Tag::Paragraph => tag::PARAGRAPH,
        Tag::Heading { .. } => tag::HEADING,
        Tag::BlockQuote(_) => tag::BLOCK_QUOTE,
        Tag::CodeBlock(_) => tag::CODE_BLOCK,
        Tag::HtmlBlock => tag::HTML_BLOCK,
        Tag::List(_) => tag::LIST,
        Tag::Item => tag::ITEM,
        Tag::FootnoteDefinition(_) => tag::FOOTNOTE_DEFINITION,
        Tag::Table(_) => tag::TABLE,
        Tag::TableHead => tag::TABLE_HEAD,
        Tag::TableRow => tag::TABLE_ROW,
        Tag::TableCell => tag::TABLE_CELL,
        Tag::Emphasis => tag::EMPHASIS,
        Tag::Strong => tag::STRONG,
        Tag::Strikethrough => tag::STRIKETHROUGH,
        Tag::Link { .. } => tag::LINK,
        Tag::Image { .. } => tag::IMAGE,
        Tag::MetadataBlock(_) => tag::METADATA_BLOCK,
        _ => tag::OTHER,
    }
}

fn tag_end_id(tag: &TagEnd) -> u32 {
    match tag {
        TagEnd::Paragraph => tag::PARAGRAPH,
        TagEnd::Heading(_) => tag::HEADING,
        TagEnd::BlockQuote(_) => tag::BLOCK_QUOTE,
        TagEnd::CodeBlock => tag::CODE_BLOCK,
        TagEnd::HtmlBlock => tag::HTML_BLOCK,
        TagEnd::List(_) => tag::LIST,
        TagEnd::Item => tag::ITEM,
        TagEnd::FootnoteDefinition => tag::FOOTNOTE_DEFINITION,
        TagEnd::Table => tag::TABLE,
        TagEnd::TableHead => tag::TABLE_HEAD,
        TagEnd::TableRow => tag::TABLE_ROW,
        TagEnd::TableCell => tag::TABLE_CELL,
        TagEnd::Emphasis => tag::EMPHASIS,
        TagEnd::Strong => tag::STRONG,
        TagEnd::Strikethrough => tag::STRIKETHROUGH,
        TagEnd::Link => tag::LINK,
        TagEnd::Image => tag::IMAGE,
        TagEnd::MetadataBlock(_) => tag::METADATA_BLOCK,
        _ => tag::OTHER,
    }
}

fn block_quote_kind(kind: &Option<BlockQuoteKind>) -> u32 {
    match kind {
        None => 0,
        Some(BlockQuoteKind::Note) => 1,
        Some(BlockQuoteKind::Tip) => 2,
        Some(BlockQuoteKind::Important) => 3,
        Some(BlockQuoteKind::Warning) => 4,
        Some(BlockQuoteKind::Caution) => 5,
    }
}

pub fn parse(src: &str, options: u32) -> Output {
    let force_strings = options & FORCE_STRINGS != 0;
    let index = Utf16Index::new(src);
    let mut out = Output::default();
    let parser = Parser::new_ext(src, Options::from_bits_truncate(options));
    for (event, range) in parser.into_offset_iter() {
        let start = index.get(range.start);
        let end = index.get(range.end);
        // Payloads equal to the source slice are not stored; JS slices the source.
        let text_like = |out: &mut Output, s: &str| -> (u32, u32) {
            let verbatim = src.get(range.clone()) == Some(s);
            let aux = if verbatim && !force_strings { NONE } else { out.push_str(s) };
            (if verbatim { VERBATIM } else { 0 }, aux)
        };
        match event {
            Event::Start(tag) => {
                let (small, aux) = match &tag {
                    Tag::Heading { level, .. } => (*level as u32, NONE),
                    Tag::BlockQuote(kind) => (block_quote_kind(kind), NONE),
                    Tag::CodeBlock(CodeBlockKind::Fenced(info)) => (1, out.push_str(info)),
                    Tag::CodeBlock(CodeBlockKind::Indented) => (0, NONE),
                    Tag::List(first) => (0, first.map_or(0, |n| n as u32 + 1)),
                    Tag::FootnoteDefinition(label) => (0, out.push_str(label)),
                    Tag::Table(alignments) => {
                        let spec: String = alignments
                            .iter()
                            .map(|a| match a {
                                Alignment::None => 'n',
                                Alignment::Left => 'l',
                                Alignment::Center => 'c',
                                Alignment::Right => 'r',
                            })
                            .collect();
                        (0, out.push_str(&spec))
                    }
                    Tag::Link { dest_url, title, .. } | Tag::Image { dest_url, title, .. } => {
                        let dest = out.push_str(dest_url);
                        out.push_str(title);
                        (0, dest)
                    }
                    _ => (0, NONE),
                };
                out.push(op::START, tag_id(&tag), small, start, end, aux);
            }
            Event::End(tag) => out.push(op::END, tag_end_id(&tag), 0, start, end, NONE),
            Event::Text(s) => {
                let (small, aux) = text_like(&mut out, &s);
                out.push(op::TEXT, 0, small, start, end, aux);
            }
            Event::Html(s) => {
                let (small, aux) = text_like(&mut out, &s);
                out.push(op::HTML, 0, small, start, end, aux);
            }
            Event::InlineHtml(s) => {
                let (small, aux) = text_like(&mut out, &s);
                out.push(op::INLINE_HTML, 0, small, start, end, aux);
            }
            // Ranges of these include delimiters, so the value is always stored.
            Event::Code(s) => {
                let aux = out.push_str(&s);
                out.push(op::CODE, 0, 0, start, end, aux);
            }
            Event::InlineMath(s) => {
                let aux = out.push_str(&s);
                out.push(op::INLINE_MATH, 0, 0, start, end, aux);
            }
            Event::DisplayMath(s) => {
                let aux = out.push_str(&s);
                out.push(op::DISPLAY_MATH, 0, 0, start, end, aux);
            }
            Event::FootnoteReference(label) => {
                let aux = out.push_str(&label);
                out.push(op::FOOTNOTE_REFERENCE, 0, 0, start, end, aux);
            }
            Event::SoftBreak => out.push(op::SOFT_BREAK, 0, 0, start, end, NONE),
            Event::HardBreak => out.push(op::HARD_BREAK, 0, 0, start, end, NONE),
            Event::Rule => out.push(op::RULE, 0, 0, start, end, NONE),
            Event::TaskListMarker(checked) => {
                out.push(op::TASK_LIST_MARKER, 0, 0, start, end, checked as u32)
            }
        }
    }
    out
}

/// Allocates `len` bytes for the input; free with `md_free`.
#[no_mangle]
pub extern "C" fn md_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// # Safety
/// `ptr`/`cap` must come from `md_alloc`.
#[no_mangle]
pub unsafe extern "C" fn md_free(ptr: *mut u8, cap: usize) {
    drop(Vec::from_raw_parts(ptr, 0, cap));
}

/// Parses `len` bytes of UTF-8 at `ptr`. Returns a header
/// `[entries_ptr, entries_len, strings_ptr, strings_len, ends_ptr, ends_len]`
/// (lengths in elements); free with `md_free_result`.
///
/// # Safety
/// `ptr..ptr + len` must be valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn md_parse(ptr: *const u8, len: usize, options: u32) -> *mut [u32; 6] {
    let src = std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len));
    let mut out = parse(src, options);
    // md_free_result rebuilds the Vecs with capacity == len.
    out.entries.shrink_to_fit();
    out.strings.shrink_to_fit();
    out.string_ends.shrink_to_fit();
    let header = Box::new([
        out.entries.as_ptr() as u32,
        out.entries.len() as u32,
        out.strings.as_ptr() as u32,
        out.strings.len() as u32,
        out.string_ends.as_ptr() as u32,
        out.string_ends.len() as u32,
    ]);
    std::mem::forget(out);
    Box::into_raw(header)
}

/// # Safety
/// `header` must come from `md_parse` and be freed once.
#[no_mangle]
pub unsafe extern "C" fn md_free_result(header: *mut [u32; 6]) {
    let h = Box::from_raw(header);
    drop(Vec::from_raw_parts(h[0] as *mut u32, h[1] as usize, h[1] as usize));
    drop(Vec::from_raw_parts(h[2] as *mut u8, h[3] as usize, h[3] as usize));
    drop(Vec::from_raw_parts(h[4] as *mut u32, h[5] as usize, h[5] as usize));
}

#[cfg(test)]
mod tests {
    use super::*;

    const GFM_CJK: u32 = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 18);

    struct Event {
        op: u32,
        tag: u32,
        small: u32,
        start: usize,
        end: usize,
        aux: u32,
    }

    fn events(out: &Output) -> Vec<Event> {
        out.entries
            .chunks(4)
            .map(|e| Event {
                op: e[0] >> 24,
                tag: (e[0] >> 16) & 0xff,
                small: e[0] & 0xffff,
                start: e[1] as usize,
                end: e[2] as usize,
                aux: e[3],
            })
            .collect()
    }

    fn utf16_slice(src: &str, start: usize, end: usize) -> String {
        let units: Vec<u16> = src.encode_utf16().collect();
        String::from_utf16(&units[start..end]).unwrap()
    }

    #[test]
    fn utf16_index_matches_encode_utf16_at_every_boundary() {
        // Mixed 1/2/3/4-byte chars between ASCII runs, so checkpoints cover
        // both pure-ASCII chunks and chunks that need scanning.
        let src = ["x".repeat(130), "a中😀é\n".repeat(12), "y".repeat(64)].concat().repeat(8);
        let index = Utf16Index::new(&src);
        let ascii = index.checkpoints.iter().filter(|&&c| c & ASCII_CHUNK != 0).count();
        assert!(ascii > 0 && ascii < index.checkpoints.len() - 1);
        for (byte, _) in src.char_indices().chain([(src.len(), ' ')]) {
            assert_eq!(index.get(byte) as usize, src[..byte].encode_utf16().count(), "byte {byte}");
        }
    }

    #[test]
    fn verbatim_payloads_equal_utf16_source_slices() {
        let src = "# 标题 😀\n\n- 中文 **粗体** [链接](https://e.com \"t\")\n- `code` &amp; \\* <b>x</b>\n\n> 引用\n> 第二行\n\n| a | 表 |\n|:-|-:|\n| 1 | 2 |\n";
        let out = parse(src, GFM_CJK | FORCE_STRINGS);
        let mut verbatim = 0;
        for e in events(&out) {
            assert!(e.start <= e.end && e.end <= src.encode_utf16().count());
            if e.small == VERBATIM && matches!(e.op, op::TEXT | op::HTML | op::INLINE_HTML) {
                verbatim += 1;
                assert_eq!(utf16_slice(src, e.start, e.end), out.string(e.aux));
            }
        }
        assert!(verbatim > 10);
    }

    #[test]
    fn non_verbatim_text_stores_decoded_payload() {
        let src = "a &amp; \\*b";
        let out = parse(src, GFM_CJK);
        let decoded: Vec<_> = events(&out)
            .into_iter()
            .filter(|e| e.op == op::TEXT && e.small != VERBATIM)
            .map(|e| (utf16_slice(src, e.start, e.end), out.string(e.aux).to_owned()))
            .collect();
        assert_eq!(decoded, [("&amp;".to_owned(), "&".to_owned())]);
        let verbatim_without_string = events(&out)
            .iter()
            .filter(|e| e.op == op::TEXT && e.small == VERBATIM)
            .all(|e| e.aux == NONE);
        assert!(verbatim_without_string);
    }

    #[test]
    fn cjk_friendly_emphasis_is_enabled_by_its_option() {
        let src = "这是**“强调”**文字";
        let strong = |options| events(&parse(src, options)).iter().any(|e| e.tag == tag::STRONG);
        assert!(strong(GFM_CJK));
        assert!(!strong(GFM_CJK & !(1 << 18)));
    }

    #[test]
    fn start_attributes_are_encoded() {
        let src = "3. x\n\n## h\n\n```ts meta\nc\n```\n\n| a | b | c |\n|:-|:-:|-:|\n";
        let out = parse(src, GFM_CJK);
        let starts: Vec<_> = events(&out).into_iter().filter(|e| e.op == op::START).collect();
        let find = |t| starts.iter().find(|e| e.tag == t).unwrap();
        assert_eq!(find(tag::LIST).aux, 4);
        assert_eq!(find(tag::HEADING).small, 2);
        assert_eq!(out.string(find(tag::CODE_BLOCK).aux), "ts meta");
        assert_eq!(out.string(find(tag::TABLE).aux), "lcr");
    }
}

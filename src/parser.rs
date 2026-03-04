/// Semantic style for a span of markdown text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MdStyle {
    // Basic markdown
    pub heading_level: u8,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub code_block: bool,
    pub blockquote: bool,
    pub list_bullet: bool,
    pub link_text: bool,
    pub link_url: bool,

    // Extended markdown
    pub strikethrough: bool,
    pub task_checkbox: bool,
    pub task_checked: bool,
    pub table_pipe: bool,
    pub table_align: bool,
    pub footnote_ref: bool,
    pub footnote_def: bool,
    pub highlight: bool,
    pub heading_id: bool,
    pub emoji_shortcode: bool,

    // Meta
    pub syntax_marker: bool,
}

/// A span of text with associated style.
#[derive(Debug, Clone)]
pub struct MdSpan {
    pub text: String,
    pub style: MdStyle,
}

/// Parse markdown text into styled spans.
/// Critical invariant: concatenation of all span texts == input.
pub fn parse_markdown(input: &str) -> Vec<MdSpan> {
    let mut spans = Vec::new();
    let mut in_code_block = false;

    for line in LineIter::new(input) {
        if in_code_block {
            if is_code_fence(line) {
                push(&mut spans, line, MdStyle {
                    code_block: true,
                    syntax_marker: true,
                    ..Default::default()
                });
                in_code_block = false;
            } else {
                push(&mut spans, line, MdStyle {
                    code_block: true,
                    ..Default::default()
                });
            }
            continue;
        }

        if is_code_fence(line) {
            push(&mut spans, line, MdStyle {
                code_block: true,
                syntax_marker: true,
                ..Default::default()
            });
            in_code_block = true;
            continue;
        }

        parse_line(line, &mut spans);
    }

    spans
}

/// Iterator that yields lines preserving their trailing newline.
struct LineIter<'a> {
    rest: &'a str,
}

impl<'a> LineIter<'a> {
    fn new(s: &'a str) -> Self {
        Self { rest: s }
    }
}

impl<'a> Iterator for LineIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        match self.rest.find('\n') {
            Some(i) => {
                let line = &self.rest[..=i];
                self.rest = &self.rest[i + 1..];
                Some(line)
            }
            None => {
                let line = self.rest;
                self.rest = "";
                Some(line)
            }
        }
    }
}

fn is_code_fence(line: &str) -> bool {
    let trimmed = line.trim_end();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn push(spans: &mut Vec<MdSpan>, text: &str, style: MdStyle) {
    if !text.is_empty() {
        spans.push(MdSpan {
            text: text.to_string(),
            style,
        });
    }
}

fn parse_line(line: &str, spans: &mut Vec<MdSpan>) {
    let content = line.trim_end_matches('\n');

    // Check for table alignment row: |:---|:---:|---:|
    if is_table_align_row(content) {
        push(spans, line, MdStyle {
            table_align: true,
            syntax_marker: true,
            ..Default::default()
        });
        return;
    }

    // Check block-level prefix
    let (prefix, body, mut base_style) = parse_block_prefix(content);

    if !prefix.is_empty() {
        let mut marker_style = base_style.clone();
        marker_style.syntax_marker = true;
        push(spans, prefix, marker_style);
    }

    // Check for heading ID suffix: {#some-id}
    let (body, heading_id_suffix) = if base_style.heading_level > 0 {
        extract_heading_id(body)
    } else {
        (body, None)
    };

    // Check if this is a table row (contains |)
    let is_table = base_style.heading_level == 0
        && !base_style.blockquote
        && !base_style.list_bullet
        && !base_style.task_checkbox
        && !base_style.footnote_def
        && content.contains('|');

    if is_table {
        parse_table_row(body, spans);
    } else {
        // Parse inline markup within the body
        parse_inline(body, &base_style, spans);
    }

    // Append heading ID suffix if present
    if let Some(id_text) = heading_id_suffix {
        push(spans, id_text, MdStyle {
            heading_id: true,
            syntax_marker: true,
            ..Default::default()
        });
    }

    // Re-append the newline if present
    if line.ends_with('\n') {
        // The newline inherits the base style (or default)
        base_style.syntax_marker = false;
        push(spans, "\n", base_style);
    }
}

fn parse_block_prefix<'a>(line: &'a str) -> (&'a str, &'a str, MdStyle) {
    // Heading: # through ######
    if let Some(level) = heading_level(line) {
        let prefix_len = level as usize + 1; // "## " = 3 chars
        if line.len() >= prefix_len {
            let prefix = &line[..prefix_len];
            let body = &line[prefix_len..];
            return (
                prefix,
                body,
                MdStyle {
                    heading_level: level,
                    ..Default::default()
                },
            );
        }
    }

    // Blockquote: > or >
    if line.starts_with("> ") {
        return (
            &line[..2],
            &line[2..],
            MdStyle {
                blockquote: true,
                ..Default::default()
            },
        );
    }
    if line.starts_with('>') && (line.len() == 1 || line.as_bytes().get(1) == Some(&b'\n')) {
        return (
            &line[..1],
            &line[1..],
            MdStyle {
                blockquote: true,
                ..Default::default()
            },
        );
    }

    // Task list: - [ ] or - [x] or - [X]
    if line.starts_with("- [ ] ") {
        return (
            &line[..6],
            &line[6..],
            MdStyle {
                task_checkbox: true,
                list_bullet: true,
                ..Default::default()
            },
        );
    }
    if line.starts_with("- [x] ") || line.starts_with("- [X] ") {
        return (
            &line[..6],
            &line[6..],
            MdStyle {
                task_checkbox: true,
                task_checked: true,
                list_bullet: true,
                ..Default::default()
            },
        );
    }

    // Footnote definition: [^id]:
    if line.starts_with("[^") {
        if let Some(close) = line.find("]:") {
            let prefix_end = close + 2;
            let prefix_end = if line.as_bytes().get(prefix_end) == Some(&b' ') {
                prefix_end + 1
            } else {
                prefix_end
            };
            return (
                &line[..prefix_end],
                &line[prefix_end..],
                MdStyle {
                    footnote_def: true,
                    ..Default::default()
                },
            );
        }
    }

    // Definition list: starts with ": "
    if line.starts_with(": ") {
        return (
            &line[..2],
            &line[2..],
            MdStyle {
                blockquote: true, // style similar to blockquote
                ..Default::default()
            },
        );
    }

    // List item: - or * (with optional leading whitespace)
    let stripped = line.trim_start();
    let indent = line.len() - stripped.len();
    if (stripped.starts_with("- ") || stripped.starts_with("* ")) && indent <= 12 {
        let prefix_end = indent + 2;
        return (
            &line[..prefix_end],
            &line[prefix_end..],
            MdStyle {
                list_bullet: true,
                ..Default::default()
            },
        );
    }

    // No block prefix
    ("", line, MdStyle::default())
}

fn heading_level(line: &str) -> Option<u8> {
    let bytes = line.as_bytes();
    let mut level = 0u8;
    for &b in bytes {
        if b == b'#' {
            level += 1;
            if level > 6 {
                return None;
            }
        } else if b == b' ' && level > 0 {
            return Some(level);
        } else {
            return None;
        }
    }
    None
}

fn extract_heading_id<'a>(body: &'a str) -> (&'a str, Option<&'a str>) {
    // Look for trailing {#some-id}
    if let Some(start) = body.rfind(" {#") {
        if body.ends_with('}') {
            return (&body[..start], Some(&body[start..]));
        }
    }
    (body, None)
}

fn is_table_align_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('-') {
        return false;
    }
    for ch in trimmed.chars() {
        if !matches!(ch, '|' | '-' | ':' | ' ') {
            return false;
        }
    }
    // Must have at least 3 dashes in sequence somewhere
    trimmed.contains("---")
}

fn parse_table_row(body: &str, spans: &mut Vec<MdSpan>) {
    let mut i = 0;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'|' {
            push(spans, &body[i..i + 1], MdStyle {
                table_pipe: true,
                syntax_marker: true,
                ..Default::default()
            });
            i += 1;
        } else {
            // Find next pipe or end
            let start = i;
            while i < bytes.len() && bytes[i] != b'|' {
                i += 1;
            }
            push(spans, &body[start..i], MdStyle::default());
        }
    }
}

/// Parse inline markdown constructs within a body of text.
fn parse_inline(body: &str, base_style: &MdStyle, spans: &mut Vec<MdSpan>) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut plain_start = 0;

    while i < len {
        // Inline code: `text`
        if bytes[i] == b'`' {
            if let Some(end) = find_closing(body, i + 1, "`") {
                flush_plain(body, plain_start, i, base_style, spans);
                let mut marker = base_style.clone();
                marker.code = true;
                marker.syntax_marker = true;
                push(spans, &body[i..i + 1], marker);
                let mut code = base_style.clone();
                code.code = true;
                push(spans, &body[i + 1..end], code);
                let mut marker2 = base_style.clone();
                marker2.code = true;
                marker2.syntax_marker = true;
                push(spans, &body[end..end + 1], marker2);
                i = end + 1;
                plain_start = i;
                continue;
            }
        }

        // Bold: **text**
        if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            if let Some(end) = find_closing(body, i + 2, "**") {
                flush_plain(body, plain_start, i, base_style, spans);
                let mut marker = base_style.clone();
                marker.bold = true;
                marker.syntax_marker = true;
                push(spans, &body[i..i + 2], marker);
                let mut bold = base_style.clone();
                bold.bold = true;
                push(spans, &body[i + 2..end], bold);
                let mut marker2 = base_style.clone();
                marker2.bold = true;
                marker2.syntax_marker = true;
                push(spans, &body[end..end + 2], marker2);
                i = end + 2;
                plain_start = i;
                continue;
            }
        }

        // Italic: *text*
        if bytes[i] == b'*' && (i + 1 >= len || bytes[i + 1] != b'*') {
            if let Some(end) = find_closing_single_star(body, i + 1) {
                flush_plain(body, plain_start, i, base_style, spans);
                let mut marker = base_style.clone();
                marker.italic = true;
                marker.syntax_marker = true;
                push(spans, &body[i..i + 1], marker);
                let mut ital = base_style.clone();
                ital.italic = true;
                push(spans, &body[i + 1..end], ital);
                let mut marker2 = base_style.clone();
                marker2.italic = true;
                marker2.syntax_marker = true;
                push(spans, &body[end..end + 1], marker2);
                i = end + 1;
                plain_start = i;
                continue;
            }
        }

        // Strikethrough: ~~text~~
        if i + 1 < len && bytes[i] == b'~' && bytes[i + 1] == b'~' {
            if let Some(end) = find_closing(body, i + 2, "~~") {
                flush_plain(body, plain_start, i, base_style, spans);
                let mut marker = base_style.clone();
                marker.strikethrough = true;
                marker.syntax_marker = true;
                push(spans, &body[i..i + 2], marker);
                let mut strike = base_style.clone();
                strike.strikethrough = true;
                push(spans, &body[i + 2..end], strike);
                let mut marker2 = base_style.clone();
                marker2.strikethrough = true;
                marker2.syntax_marker = true;
                push(spans, &body[end..end + 2], marker2);
                i = end + 2;
                plain_start = i;
                continue;
            }
        }

        // Highlight: ==text==
        if i + 1 < len && bytes[i] == b'=' && bytes[i + 1] == b'=' {
            if let Some(end) = find_closing(body, i + 2, "==") {
                flush_plain(body, plain_start, i, base_style, spans);
                let mut marker = base_style.clone();
                marker.highlight = true;
                marker.syntax_marker = true;
                push(spans, &body[i..i + 2], marker);
                let mut hl = base_style.clone();
                hl.highlight = true;
                push(spans, &body[i + 2..end], hl);
                let mut marker2 = base_style.clone();
                marker2.highlight = true;
                marker2.syntax_marker = true;
                push(spans, &body[end..end + 2], marker2);
                i = end + 2;
                plain_start = i;
                continue;
            }
        }

        // Footnote reference: [^id]
        if bytes[i] == b'[' && i + 1 < len && bytes[i + 1] == b'^' {
            if let Some(close) = body[i + 2..].find(']') {
                let end = i + 2 + close;
                // Make sure it's not a footnote definition (no colon after])
                if end + 1 >= len || bytes[end + 1] != b':' {
                    flush_plain(body, plain_start, i, base_style, spans);
                    push(spans, &body[i..end + 1], MdStyle {
                        footnote_ref: true,
                        syntax_marker: true,
                        ..Default::default()
                    });
                    i = end + 1;
                    plain_start = i;
                    continue;
                }
            }
        }

        // Link: [text](url)
        if bytes[i] == b'[' {
            if let Some((text_end, url_end)) = find_link(body, i) {
                flush_plain(body, plain_start, i, base_style, spans);
                // [
                push(spans, &body[i..i + 1], MdStyle {
                    link_text: true,
                    syntax_marker: true,
                    ..Default::default()
                });
                // text
                push(spans, &body[i + 1..text_end], MdStyle {
                    link_text: true,
                    ..Default::default()
                });
                // ](
                push(spans, &body[text_end..text_end + 2], MdStyle {
                    link_url: true,
                    syntax_marker: true,
                    ..Default::default()
                });
                // url
                push(spans, &body[text_end + 2..url_end], MdStyle {
                    link_url: true,
                    ..Default::default()
                });
                // )
                push(spans, &body[url_end..url_end + 1], MdStyle {
                    link_url: true,
                    syntax_marker: true,
                    ..Default::default()
                });
                i = url_end + 1;
                plain_start = i;
                continue;
            }
        }

        // Emoji shortcode: :name:
        if bytes[i] == b':' {
            if let Some(end) = find_emoji_shortcode(body, i) {
                flush_plain(body, plain_start, i, base_style, spans);
                push(spans, &body[i..end + 1], MdStyle {
                    emoji_shortcode: true,
                    ..Default::default()
                });
                i = end + 1;
                plain_start = i;
                continue;
            }
        }

        // Bare URL: https:// or http://
        if bytes[i] == b'h'
            && (body[i..].starts_with("https://") || body[i..].starts_with("http://"))
        {
            let url_end = find_url_end(body, i);
            if url_end > i + 8 {
                flush_plain(body, plain_start, i, base_style, spans);
                push(spans, &body[i..url_end], MdStyle {
                    link_url: true,
                    ..Default::default()
                });
                i = url_end;
                plain_start = i;
                continue;
            }
        }

        i += 1;
    }

    // Flush remaining plain text
    flush_plain(body, plain_start, len, base_style, spans);
}

fn flush_plain(body: &str, start: usize, end: usize, base_style: &MdStyle, spans: &mut Vec<MdSpan>) {
    if start < end {
        push(spans, &body[start..end], base_style.clone());
    }
}

/// Find closing delimiter, returns the byte index of the start of the closing delimiter.
fn find_closing(text: &str, from: usize, delim: &str) -> Option<usize> {
    let search = &text[from..];
    search.find(delim).map(|pos| from + pos)
}

/// Find closing single `*` that is NOT `**`.
fn find_closing_single_star(text: &str, from: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'*' {
            // Make sure it's not **
            if i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                i += 2;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Find a markdown link: [text](url). Returns (text_end_index, url_end_index).
fn find_link(text: &str, open_bracket: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    // Find closing ]
    let mut i = open_bracket + 1;
    while i < bytes.len() && bytes[i] != b']' {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let text_end = i;
    // Must be immediately followed by (
    if text_end + 1 >= bytes.len() || bytes[text_end + 1] != b'(' {
        return None;
    }
    // Find closing )
    let mut j = text_end + 2;
    while j < bytes.len() && bytes[j] != b')' {
        j += 1;
    }
    if j >= bytes.len() {
        return None;
    }
    Some((text_end, j))
}

/// Find an emoji shortcode starting at a colon. Returns end index (inclusive of closing colon).
fn find_emoji_shortcode(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if start + 2 >= bytes.len() {
        return None;
    }
    // Must start with : followed by a letter
    if !bytes[start + 1].is_ascii_alphabetic() {
        return None;
    }
    let mut i = start + 2;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b':' {
            // Valid shortcode: at least 2 chars between colons
            if i - start >= 3 {
                return Some(i);
            }
            return None;
        }
        if b == b' ' || b == b'\n' {
            return None;
        }
        if !(b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'+') {
            return None;
        }
        i += 1;
    }
    None
}

/// Find the end of a bare URL.
fn find_url_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b' ' || b == b'\n' || b == b'\t' || b == b')' || b == b'>' || b == b'"' || b == b'\'' {
            break;
        }
        i += 1;
    }
    // Trim trailing punctuation that's likely not part of the URL
    while i > start && matches!(bytes[i - 1], b'.' | b',' | b';' | b':' | b'!' | b'?') {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concat_spans(spans: &[MdSpan]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    fn find_span<'a>(spans: &'a [MdSpan], pred: impl Fn(&MdStyle) -> bool) -> Option<&'a MdSpan> {
        spans.iter().find(|s| pred(&s.style))
    }

    // === CONCATENATION INVARIANT ===

    #[test]
    fn test_concatenation_invariant() {
        let input = "# Hello World\n\nSome **bold** and *italic* text.\n\n```\ncode block\n```\n\n- item 1\n- item 2\n\n> quote\n\n[link](https://example.com)\n\n~~deleted~~ and ==highlighted==\n\n- [ ] todo\n- [x] done\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n[^1]: footnote\n\nSee [^1] and :smile:\n\nhttps://example.com\n\n## Heading {#my-id}\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
    }

    #[test]
    fn test_empty_input() {
        let spans = parse_markdown("");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_plain_text() {
        let input = "Just plain text\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        // Should have no special styling (except default)
        for span in &spans {
            assert_eq!(span.style.heading_level, 0);
            assert!(!span.style.bold);
            assert!(!span.style.code);
        }
    }

    // === HEADINGS ===

    #[test]
    fn test_heading_h1() {
        let input = "# Hello\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        // The "# " prefix is a syntax marker
        assert!(spans[0].style.syntax_marker);
        assert_eq!(spans[0].style.heading_level, 1);
        // The "Hello" body is heading level 1
        assert_eq!(spans[1].style.heading_level, 1);
        assert!(!spans[1].style.syntax_marker);
    }

    #[test]
    fn test_heading_h3() {
        let input = "### Third\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        assert_eq!(spans[0].style.heading_level, 3);
    }

    #[test]
    fn test_heading_with_id() {
        let input = "## Title {#my-id}\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let id_span = find_span(&spans, |s| s.heading_id);
        assert!(id_span.is_some());
        assert_eq!(id_span.unwrap().text, " {#my-id}");
    }

    // === BOLD / ITALIC ===

    #[test]
    fn test_bold() {
        let input = "some **bold** text\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let bold = find_span(&spans, |s| s.bold && !s.syntax_marker);
        assert!(bold.is_some());
        assert_eq!(bold.unwrap().text, "bold");
    }

    #[test]
    fn test_italic() {
        let input = "some *italic* text\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let italic = find_span(&spans, |s| s.italic && !s.syntax_marker);
        assert!(italic.is_some());
        assert_eq!(italic.unwrap().text, "italic");
    }

    // === CODE ===

    #[test]
    fn test_inline_code() {
        let input = "use `println!` here\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let code = find_span(&spans, |s| s.code && !s.syntax_marker);
        assert!(code.is_some());
        assert_eq!(code.unwrap().text, "println!");
    }

    #[test]
    fn test_code_block() {
        let input = "```\nfn main() {}\n```\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let code = find_span(&spans, |s| s.code_block && !s.syntax_marker);
        assert!(code.is_some());
        assert_eq!(code.unwrap().text, "fn main() {}\n");
    }

    #[test]
    fn test_code_block_tilde() {
        let input = "~~~\ncode\n~~~\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let code = find_span(&spans, |s| s.code_block && !s.syntax_marker);
        assert!(code.is_some());
    }

    // === BLOCKQUOTE ===

    #[test]
    fn test_blockquote() {
        let input = "> quoted text\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let quote = find_span(&spans, |s| s.blockquote && !s.syntax_marker);
        assert!(quote.is_some());
        assert_eq!(quote.unwrap().text, "quoted text");
    }

    // === LISTS ===

    #[test]
    fn test_list_item() {
        let input = "- item one\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        assert!(spans[0].style.list_bullet);
        assert!(spans[0].style.syntax_marker);
    }

    // === LINKS ===

    #[test]
    fn test_link() {
        let input = "[click](https://example.com)\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let link_text = find_span(&spans, |s| s.link_text && !s.syntax_marker);
        assert!(link_text.is_some());
        assert_eq!(link_text.unwrap().text, "click");
        let link_url = find_span(&spans, |s| s.link_url && !s.syntax_marker);
        assert!(link_url.is_some());
        assert_eq!(link_url.unwrap().text, "https://example.com");
    }

    // === STRIKETHROUGH ===

    #[test]
    fn test_strikethrough() {
        let input = "some ~~deleted~~ text\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let strike = find_span(&spans, |s| s.strikethrough && !s.syntax_marker);
        assert!(strike.is_some());
        assert_eq!(strike.unwrap().text, "deleted");
    }

    // === HIGHLIGHT ===

    #[test]
    fn test_highlight() {
        let input = "some ==highlighted== text\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let hl = find_span(&spans, |s| s.highlight && !s.syntax_marker);
        assert!(hl.is_some());
        assert_eq!(hl.unwrap().text, "highlighted");
    }

    // === TASK LISTS ===

    #[test]
    fn test_task_unchecked() {
        let input = "- [ ] todo item\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        assert!(spans[0].style.task_checkbox);
        assert!(!spans[0].style.task_checked);
    }

    #[test]
    fn test_task_checked() {
        let input = "- [x] done item\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        assert!(spans[0].style.task_checkbox);
        assert!(spans[0].style.task_checked);
    }

    // === TABLES ===

    #[test]
    fn test_table_row() {
        let input = "| a | b |\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let pipe = find_span(&spans, |s| s.table_pipe);
        assert!(pipe.is_some());
        assert_eq!(pipe.unwrap().text, "|");
    }

    #[test]
    fn test_table_align_row() {
        let input = "|---|---|\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        assert!(spans[0].style.table_align);
    }

    // === FOOTNOTES ===

    #[test]
    fn test_footnote_def() {
        let input = "[^1]: This is a footnote\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        assert!(spans[0].style.footnote_def);
    }

    #[test]
    fn test_footnote_ref() {
        let input = "See [^1] here\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let fref = find_span(&spans, |s| s.footnote_ref);
        assert!(fref.is_some());
        assert_eq!(fref.unwrap().text, "[^1]");
    }

    // === EMOJI SHORTCODES ===

    #[test]
    fn test_emoji_shortcode() {
        let input = "hello :smile: world\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let emoji = find_span(&spans, |s| s.emoji_shortcode);
        assert!(emoji.is_some());
        assert_eq!(emoji.unwrap().text, ":smile:");
    }

    // === BARE URLS ===

    #[test]
    fn test_bare_url() {
        let input = "visit https://example.com today\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        let url = find_span(&spans, |s| s.link_url);
        assert!(url.is_some());
        assert_eq!(url.unwrap().text, "https://example.com");
    }

    // === NO NEWLINE AT END ===

    #[test]
    fn test_no_trailing_newline() {
        let input = "no newline";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
    }

    // === MULTIPLE INLINE IN ONE LINE ===

    #[test]
    fn test_mixed_inline() {
        let input = "**bold** and *italic* and `code`\n";
        let spans = parse_markdown(input);
        assert_eq!(concat_spans(&spans), input);
        assert!(find_span(&spans, |s| s.bold && !s.syntax_marker).is_some());
        assert!(find_span(&spans, |s| s.italic && !s.syntax_marker).is_some());
        assert!(find_span(&spans, |s| s.code && !s.syntax_marker).is_some());
    }
}

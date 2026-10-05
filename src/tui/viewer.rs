//! Read-only text viewing for responses: wrapping, JSON highlighting, and search.
//!
//! Only the lines inside the viewport are styled and wrapped on each frame, so large
//! responses stay responsive.

use super::{input::char_width, theme};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

const MAX_SEARCH_MATCHES: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Json,
    Plain,
    Headers,
}

/// A search hit: logical line and starting character column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub column: usize,
}

pub struct Search<'a> {
    pub query: &'a str,
    pub current: Option<Match>,
}

/// Makes text safe to place in terminal cells: tabs become spaces, other control
/// characters become a visible replacement character.
pub fn sanitize(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .split('\n')
        .map(|line| {
            line.chars()
                .flat_map(|character| match character {
                    '\t' => vec![' '; 4],
                    '\r' => vec![],
                    c if c.is_control() => vec![char::REPLACEMENT_CHARACTER],
                    c => vec![c],
                })
                .collect()
        })
        .collect()
}

/// Number of terminal rows `line` occupies when wrapped at `width` columns.
pub fn visual_height(line: &str, width: usize) -> usize {
    if width == 0 {
        return 1;
    }
    if line.is_ascii() {
        return line.len().div_ceil(width).max(1);
    }
    let mut rows = 1;
    let mut used = 0;
    for character in line.chars() {
        let character_width = char_width(character);
        if used + character_width > width && used > 0 {
            rows += 1;
            used = 0;
        }
        used += character_width;
    }
    rows
}

pub fn total_height(lines: &[String], width: usize) -> usize {
    lines.iter().map(|line| visual_height(line, width)).sum()
}

/// Visual row at which logical `line` starts, plus the row of `column` within it.
pub fn visual_row_of(lines: &[String], width: usize, target: Match) -> usize {
    let before = lines
        .iter()
        .take(target.line)
        .map(|line| visual_height(line, width))
        .sum::<usize>();
    let within = lines
        .get(target.line)
        .map(|line| {
            let prefix = line.chars().take(target.column).collect::<String>();
            visual_height(&prefix, width).saturating_sub(1)
        })
        .unwrap_or_default();
    before + within
}

/// Wrapped, styled rows `[scroll, scroll + height)` of `lines`.
pub fn render_window(
    lines: &[String],
    kind: ContentKind,
    width: usize,
    scroll: usize,
    height: usize,
    search: Option<&Search<'_>>,
) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut output = Vec::with_capacity(height);
    let mut row = 0;

    for (index, line) in lines.iter().enumerate() {
        if output.len() >= height {
            break;
        }
        let line_height = visual_height(line, width);
        if row + line_height <= scroll {
            row += line_height;
            continue;
        }
        let skip_rows = scroll.saturating_sub(row);
        let take_rows = height - output.len();
        output.extend(render_line(
            index, line, kind, width, skip_rows, take_rows, search,
        ));
        row += line_height;
    }

    output
}

fn render_line(
    index: usize,
    line: &str,
    kind: ContentKind,
    width: usize,
    skip_rows: usize,
    take_rows: usize,
    search: Option<&Search<'_>>,
) -> Vec<Line<'static>> {
    let chars = line.chars().collect::<Vec<_>>();
    let styles = line_styles(index, &chars, kind, search);

    // ASCII rows map to fixed character ranges, so a huge single-line body only processes
    // the visible slice.
    if line.is_ascii() {
        let start = (skip_rows * width).min(chars.len());
        let end = ((skip_rows + take_rows) * width).min(chars.len());
        let rows = wrap(
            &chars[start..end],
            styles.as_ref().map(|styles| &styles[start..end]),
            width,
        );
        return rows.into_iter().take(take_rows).collect();
    }

    wrap(&chars, styles.as_deref(), width)
        .into_iter()
        .skip(skip_rows)
        .take(take_rows)
        .collect()
}

fn line_styles(
    index: usize,
    chars: &[char],
    kind: ContentKind,
    search: Option<&Search<'_>>,
) -> Option<Vec<Style>> {
    let mut styles = match kind {
        ContentKind::Json => Some(json_styles(chars)),
        ContentKind::Headers => Some(header_styles(chars)),
        ContentKind::Plain => None,
    };

    if let Some(search) = search.filter(|search| !search.query.is_empty()) {
        let query = lowercase(&search.query.chars().collect::<Vec<_>>());
        let haystack = lowercase(chars);
        let mut start = 0;
        while let Some(column) = find(&haystack, &query, start) {
            let styles = styles.get_or_insert_with(|| vec![theme::text(); chars.len()]);
            let current = search.current == Some(Match {
                line: index,
                column,
            });
            let style = if current {
                theme::search_current()
            } else {
                theme::search_match()
            };
            styles[column..column + query.len()].fill(style);
            start = column + query.len().max(1);
        }
    }

    styles
}

fn wrap(chars: &[char], styles: Option<&[Style]>, width: usize) -> Vec<Line<'static>> {
    let mut rows = Vec::new();
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut buffer_style = theme::text();
    let mut used = 0;

    for (position, character) in chars.iter().enumerate() {
        let style = styles
            .map(|styles| styles[position])
            .unwrap_or_else(theme::text);
        let character_width = char_width(*character);
        if used + character_width > width && used > 0 {
            if !buffer.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buffer), buffer_style));
            }
            rows.push(Line::from(std::mem::take(&mut spans)));
            used = 0;
        }
        if style != buffer_style && !buffer.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut buffer), buffer_style));
        }
        buffer_style = style;
        buffer.push(*character);
        used += character_width;
    }

    if !buffer.is_empty() {
        spans.push(Span::styled(buffer, buffer_style));
    }
    rows.push(Line::from(spans));
    rows
}

fn json_styles(chars: &[char]) -> Vec<Style> {
    let mut styles = vec![theme::text(); chars.len()];
    let mut index = 0;

    while index < chars.len() {
        let character = chars[index];
        if character == '"' {
            let start = index;
            index += 1;
            while index < chars.len() {
                match chars[index] {
                    '\\' => index += 2,
                    '"' => {
                        index += 1;
                        break;
                    }
                    _ => index += 1,
                }
            }
            let end = index.min(chars.len());
            let next = chars[end..].iter().find(|c| !c.is_whitespace());
            let style = if next == Some(&':') {
                theme::json_key()
            } else {
                theme::json_string()
            };
            styles[start..end].fill(style);
            index = end;
        } else if character == '-' || character.is_ascii_digit() {
            let start = index;
            while index < chars.len()
                && (chars[index].is_ascii_digit() || matches!(chars[index], '-' | '+' | '.' | 'e' | 'E'))
            {
                index += 1;
            }
            styles[start..index].fill(theme::json_number());
        } else if let Some(length) = ["true", "false", "null"]
            .iter()
            .find(|literal| starts_with(chars, index, literal))
            .map(|literal| literal.len())
        {
            styles[index..index + length].fill(theme::json_literal());
            index += length;
        } else {
            if matches!(character, '{' | '}' | '[' | ']' | ',' | ':') {
                styles[index] = theme::json_punctuation();
            }
            index += 1;
        }
    }

    styles
}

fn header_styles(chars: &[char]) -> Vec<Style> {
    let mut styles = vec![theme::text(); chars.len()];
    if let Some(colon) = chars.iter().position(|c| *c == ':') {
        styles[..colon].fill(theme::json_key());
        styles[colon] = theme::json_punctuation();
    }
    styles
}

fn starts_with(chars: &[char], index: usize, literal: &str) -> bool {
    let literal = literal.chars().collect::<Vec<_>>();
    chars.len() >= index + literal.len() && chars[index..index + literal.len()] == literal[..]
}

fn lowercase(chars: &[char]) -> Vec<char> {
    chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect()
}

fn find(haystack: &[char], needle: &[char], start: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (start..=haystack.len() - needle.len()).find(|&index| haystack[index..index + needle.len()] == *needle)
}

/// All case-insensitive occurrences of `query`, in reading order.
pub fn search_matches(lines: &[String], query: &str) -> Vec<Match> {
    let needle = lowercase(&query.chars().collect::<Vec<_>>());
    if needle.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let haystack = lowercase(&line.chars().collect::<Vec<_>>());
        let mut start = 0;
        while let Some(column) = find(&haystack, &needle, start) {
            matches.push(Match {
                line: line_index,
                column,
            });
            if matches.len() >= MAX_SEARCH_MATCHES {
                return matches;
            }
            start = column + needle.len();
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lines: &[Line<'_>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn wraps_and_windows_long_lines() {
        let lines = vec![String::from("abcdefghij"), String::from("xyz")];
        assert_eq!(total_height(&lines, 4), 4);

        let window = render_window(&lines, ContentKind::Plain, 4, 1, 2, None);
        assert_eq!(text(&window), vec!["efgh", "ij"]);
    }

    #[test]
    fn non_ascii_wrapping_matches_visual_height() {
        let line = String::from("ééééé");
        assert_eq!(visual_height(&line, 2), 3);
        let window = render_window(&[line], ContentKind::Plain, 2, 0, 5, None);
        assert_eq!(text(&window), vec!["éé", "éé", "é"]);
    }

    #[test]
    fn json_keys_and_values_get_distinct_styles() {
        let chars = "  \"id\": 42, \"ok\": true".chars().collect::<Vec<_>>();
        let styles = json_styles(&chars);
        assert_eq!(styles[2], theme::json_key());
        assert_eq!(styles[8], theme::json_number());
        assert_eq!(styles[18], theme::json_literal());
    }

    #[test]
    fn search_finds_case_insensitive_matches_and_rows() {
        let lines = vec![String::from("Alpha beta"), String::from("BETA")];
        let matches = search_matches(&lines, "beta");
        assert_eq!(
            matches,
            vec![
                Match { line: 0, column: 6 },
                Match { line: 1, column: 0 }
            ]
        );
        assert_eq!(visual_row_of(&lines, 4, matches[0]), 1);
        assert_eq!(visual_row_of(&lines, 4, matches[1]), 3);
    }

    #[test]
    fn sanitize_replaces_control_characters() {
        assert_eq!(sanitize("a\tb\r\nc\u{7}"), vec!["a    b", "c\u{FFFD}"]);
    }
}

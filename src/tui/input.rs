use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::cell::Cell;
use unicode_width::UnicodeWidthChar;

const INDENT: &str = "  ";
/// Columns a tab occupies on screen; tabs stay tabs in the text itself.
const TAB_WIDTH: usize = 4;
const PAGE_ROWS: usize = 10;

/// An editable text buffer with readline-style editing keys.
///
/// The scroll offsets are interior-mutable so rendering can keep the cursor in view without
/// needing mutable access to the application state.
#[derive(Debug, Clone)]
pub struct TextInput {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    multiline: bool,
    scroll: Cell<(usize, usize)>,
    viewport_rows: Cell<usize>,
    /// Kept from loaded text so values round-trip exactly (`\r\n` bodies stay `\r\n`).
    line_ending: &'static str,
}

impl PartialEq for TextInput {
    fn eq(&self, other: &Self) -> bool {
        self.multiline == other.multiline && self.lines == other.lines
    }
}

impl Eq for TextInput {}

impl TextInput {
    pub fn single(value: impl Into<String>) -> TextInput {
        TextInput::with_mode(value.into(), false)
    }

    pub fn multiline(value: impl Into<String>) -> TextInput {
        TextInput::with_mode(value.into(), true)
    }

    fn with_mode(value: String, multiline: bool) -> TextInput {
        let mut input = TextInput {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            multiline,
            scroll: Cell::new((0, 0)),
            viewport_rows: Cell::new(PAGE_ROWS),
            line_ending: "\n",
        };
        input.set_value(value);
        input
    }

    pub fn value(&self) -> String {
        self.lines.join(self.line_ending)
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(String::is_empty)
    }

    /// Replaces the content and moves the cursor to the end.
    /// Replaces the content and moves the cursor to the end. Multiline text is kept exactly,
    /// tabs and line endings included, so loading and saving never alters it.
    pub fn set_value(&mut self, value: String) {
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll.set((0, 0));
        if self.multiline {
            self.line_ending = if value.contains("\r\n") { "\r\n" } else { "\n" };
            self.lines = value
                .split('\n')
                .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
                .collect();
            self.cursor_row = self.lines.len() - 1;
            self.cursor_col = self.line_len(self.cursor_row);
        } else {
            self.lines = vec![String::new()];
            self.insert_str(&value);
        }
    }

    pub fn clear(&mut self) {
        self.set_value(String::new());
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Inserts text at the cursor, as when pasting. Single-line inputs flatten newlines and
    /// drop trailing ones, so a pasted URL never carries a stray line break.
    pub fn insert_str(&mut self, text: &str) {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        if self.multiline {
            for (index, part) in normalized.split('\n').enumerate() {
                if index > 0 {
                    self.split_line_at_cursor(String::new());
                }
                self.insert_plain(part);
            }
        } else {
            let flattened = normalized
                .trim_end_matches('\n')
                .replace(['\n', '\t'], " ");
            self.insert_plain(&flattened);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            KeyCode::Char(character) if ctrl && !alt => match character {
                'a' => self.move_line_start(),
                'e' => self.move_line_end(),
                'u' => self.delete_to_line_start(),
                'k' => self.delete_to_line_end(),
                'w' => self.delete_word_back(),
                'h' => self.backspace(),
                _ => return false,
            },
            KeyCode::Char(character) if alt && !ctrl => match character {
                'b' => self.move_word_left(),
                'f' => self.move_word_right(),
                'd' => self.delete_word_forward(),
                _ => return false,
            },
            // Plain characters, plus Ctrl+Alt combinations that AltGr layouts use for symbols.
            KeyCode::Char(character) => self.insert_char(character),
            KeyCode::Backspace if ctrl || alt => self.delete_word_back(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete if ctrl || alt => self.delete_word_forward(),
            KeyCode::Delete => self.delete(),
            KeyCode::Enter if self.multiline => self.insert_newline(),
            KeyCode::Left if ctrl || alt => self.move_word_left(),
            KeyCode::Left => self.move_left(),
            KeyCode::Right if ctrl || alt => self.move_word_right(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up if self.multiline => self.move_vertical(-1),
            KeyCode::Down if self.multiline => self.move_vertical(1),
            KeyCode::PageUp if self.multiline => {
                self.move_vertical(-(self.viewport_rows.get().max(1) as isize))
            }
            KeyCode::PageDown if self.multiline => {
                self.move_vertical(self.viewport_rows.get().max(1) as isize)
            }
            KeyCode::Home if ctrl && self.multiline => {
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            KeyCode::End if ctrl && self.multiline => {
                self.cursor_row = self.lines.len() - 1;
                self.cursor_col = self.line_len(self.cursor_row);
            }
            KeyCode::Home => self.move_line_start(),
            KeyCode::End => self.move_line_end(),
            _ => return false,
        }
        true
    }

    /// Visible part of a single-line input that is `width` columns wide, and the cursor
    /// column within it. Scrolls horizontally so the cursor always stays visible.
    pub fn single_line_view(&self, width: usize) -> (String, usize) {
        let (row, _) = self.scroll.get();
        let col_offset = self.horizontal_offset(self.cursor_row, width);
        self.scroll.set((row, col_offset));
        let line = &self.lines[self.cursor_row];
        let cursor_x = width_of(line.chars().skip(col_offset).take(self.cursor_col - col_offset));
        (slice_by_width(line, col_offset, width), cursor_x)
    }

    /// First visible row and column for a `width` x `height` viewport, adjusted so the
    /// cursor stays inside it.
    pub fn viewport(&self, width: usize, height: usize) -> (usize, usize) {
        let height = height.max(1);
        self.viewport_rows.set(height);
        let (mut row_offset, _) = self.scroll.get();
        if self.cursor_row < row_offset {
            row_offset = self.cursor_row;
        } else if self.cursor_row >= row_offset + height {
            row_offset = self.cursor_row + 1 - height;
        }
        row_offset = row_offset.min(self.lines.len().saturating_sub(1));
        let col_offset = self.horizontal_offset(self.cursor_row, width);
        self.scroll.set((row_offset, col_offset));
        (row_offset, col_offset)
    }

    fn horizontal_offset(&self, row: usize, width: usize) -> usize {
        let (_, mut col_offset) = self.scroll.get();
        let width = width.max(1);
        let chars = self.lines[row].chars().collect::<Vec<_>>();
        let cursor = self.cursor_col.min(chars.len());
        if cursor < col_offset {
            col_offset = cursor;
        }
        // Keep one column free for the cursor when it sits at the end of the text.
        while col_offset < cursor && width_of(chars[col_offset..cursor].iter().copied()) >= width
        {
            col_offset += 1;
        }
        col_offset.min(cursor)
    }

    fn insert_plain(&mut self, text: &str) {
        for character in text.chars() {
            let row = self.cursor_row;
            let byte = byte_index(&self.lines[row], self.cursor_col);
            self.lines[row].insert(byte, character);
            self.cursor_col += 1;
        }
    }

    fn insert_char(&mut self, character: char) {
        if self.multiline && matches!(character, '}' | ']') {
            self.dedent_before_closing_bracket();
        }
        self.insert_plain(&character.to_string());
    }

    /// Typing a closing bracket on an otherwise blank line removes one indentation level.
    fn dedent_before_closing_bracket(&mut self) {
        let row = self.cursor_row;
        let before = self.lines[row]
            .chars()
            .take(self.cursor_col)
            .collect::<String>();
        if before.len() >= INDENT.len() && before.chars().all(|c| c == ' ') {
            let start = self.cursor_col - INDENT.len();
            self.remove_range(row, start, self.cursor_col);
            self.cursor_col = start;
        }
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let row = self.cursor_row;
            self.remove_range(row, self.cursor_col - 1, self.cursor_col);
            self.cursor_col -= 1;
        } else if self.multiline && self.cursor_row > 0 {
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.line_len(self.cursor_row);
            self.lines[self.cursor_row].push_str(&current);
        }
    }

    fn delete(&mut self) {
        let row = self.cursor_row;
        if self.cursor_col < self.line_len(row) {
            self.remove_range(row, self.cursor_col, self.cursor_col + 1);
        } else if self.multiline && row + 1 < self.lines.len() {
            let next = self.lines.remove(row + 1);
            self.lines[row].push_str(&next);
        }
    }

    fn insert_newline(&mut self) {
        let row = self.cursor_row;
        let line = self.lines[row].clone();
        let indent = line
            .chars()
            .take_while(|c| *c == ' ')
            .collect::<String>();
        let before = line.chars().take(self.cursor_col).collect::<String>();
        let after = line.chars().skip(self.cursor_col).collect::<String>();
        let opens_block = matches!(before.trim_end().chars().last(), Some('{' | '['));
        let closes_block = matches!(after.trim_start().chars().next(), Some('}' | ']'));

        if opens_block {
            let inner_indent = format!("{indent}{INDENT}");
            self.lines[row] = before.trim_end().to_string();
            if closes_block {
                self.lines
                    .insert(row + 1, format!("{indent}{}", after.trim_start()));
                self.lines.insert(row + 1, inner_indent.clone());
            } else {
                self.lines
                    .insert(row + 1, format!("{inner_indent}{}", after.trim_start()));
            }
            self.cursor_row = row + 1;
            self.cursor_col = inner_indent.chars().count();
        } else {
            let carried = after.trim_start().to_string();
            self.split_line_at_cursor(indent.clone());
            self.lines[self.cursor_row] = format!("{indent}{carried}");
            self.cursor_col = indent.chars().count();
        }
    }

    /// Splits the current line at the cursor, starting the new line with `prefix`.
    fn split_line_at_cursor(&mut self, prefix: String) {
        let row = self.cursor_row;
        let byte = byte_index(&self.lines[row], self.cursor_col);
        let rest = self.lines[row].split_off(byte);
        self.lines.insert(row + 1, format!("{prefix}{rest}"));
        self.cursor_row += 1;
        self.cursor_col = prefix.chars().count();
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.multiline && self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.line_len(self.cursor_row);
        }
    }

    fn move_right(&mut self) {
        if self.cursor_col < self.line_len(self.cursor_row) {
            self.cursor_col += 1;
        } else if self.multiline && self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_vertical(&mut self, delta: isize) {
        let last = self.lines.len() as isize - 1;
        let row = (self.cursor_row as isize + delta).clamp(0, last) as usize;
        self.cursor_row = row;
        self.cursor_col = self.cursor_col.min(self.line_len(row));
    }

    fn move_line_start(&mut self) {
        self.cursor_col = 0;
    }

    fn move_line_end(&mut self) {
        self.cursor_col = self.line_len(self.cursor_row);
    }

    fn move_word_left(&mut self) {
        if self.cursor_col == 0 {
            self.move_left();
            return;
        }
        self.cursor_col = self.word_start_before(self.cursor_col);
    }

    fn move_word_right(&mut self) {
        if self.cursor_col >= self.line_len(self.cursor_row) {
            self.move_right();
            return;
        }
        self.cursor_col = self.word_end_after(self.cursor_col);
    }

    fn delete_to_line_start(&mut self) {
        let row = self.cursor_row;
        self.remove_range(row, 0, self.cursor_col);
        self.cursor_col = 0;
    }

    fn delete_to_line_end(&mut self) {
        let row = self.cursor_row;
        let len = self.line_len(row);
        if self.cursor_col < len {
            self.remove_range(row, self.cursor_col, len);
        } else if self.multiline && row + 1 < self.lines.len() {
            let next = self.lines.remove(row + 1);
            self.lines[row].push_str(&next);
        }
    }

    fn delete_word_back(&mut self) {
        if self.cursor_col == 0 {
            self.backspace();
            return;
        }
        let start = self.word_start_before(self.cursor_col);
        let row = self.cursor_row;
        self.remove_range(row, start, self.cursor_col);
        self.cursor_col = start;
    }

    fn delete_word_forward(&mut self) {
        let row = self.cursor_row;
        if self.cursor_col >= self.line_len(row) {
            self.delete();
            return;
        }
        let end = self.word_end_after(self.cursor_col);
        self.remove_range(row, self.cursor_col, end);
    }

    /// Start of the word left of `col`, skipping separators first (`/`, `.`, spaces, ...).
    fn word_start_before(&self, col: usize) -> usize {
        let chars = self.lines[self.cursor_row].chars().collect::<Vec<_>>();
        let mut index = col.min(chars.len());
        while index > 0 && !is_word_char(chars[index - 1]) {
            index -= 1;
        }
        while index > 0 && is_word_char(chars[index - 1]) {
            index -= 1;
        }
        index
    }

    fn word_end_after(&self, col: usize) -> usize {
        let chars = self.lines[self.cursor_row].chars().collect::<Vec<_>>();
        let mut index = col.min(chars.len());
        while index < chars.len() && !is_word_char(chars[index]) {
            index += 1;
        }
        while index < chars.len() && is_word_char(chars[index]) {
            index += 1;
        }
        index
    }

    fn remove_range(&mut self, row: usize, start: usize, end: usize) {
        let line = &mut self.lines[row];
        let start_byte = byte_index(line, start);
        let end_byte = byte_index(line, end);
        line.replace_range(start_byte..end_byte, "");
    }

    fn line_len(&self, row: usize) -> usize {
        self.lines
            .get(row)
            .map(|line| line.chars().count())
            .unwrap_or_default()
    }
}

impl Default for TextInput {
    fn default() -> Self {
        TextInput::single("")
    }
}

fn is_word_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

/// Columns `character` occupies when drawn by [`slice_by_width`].
pub fn char_width(character: char) -> usize {
    match character {
        '\t' => TAB_WIDTH,
        c if c.is_control() => 1,
        c => c.width().unwrap_or(0),
    }
}

fn display_char(character: char, output: &mut String) {
    match character {
        '\t' => output.push_str(&" ".repeat(TAB_WIDTH)),
        c if c.is_control() => output.push(char::REPLACEMENT_CHARACTER),
        c => output.push(c),
    }
}

pub fn width_of(chars: impl Iterator<Item = char>) -> usize {
    chars.map(char_width).sum()
}

/// Drawable text for the characters of `line` from char `offset` that fit in `width`
/// columns; tabs become spaces and control characters a visible placeholder.
pub fn slice_by_width(line: &str, offset: usize, width: usize) -> String {
    let mut used = 0;
    let mut output = String::new();
    for character in line.chars().skip(offset) {
        used += char_width(character);
        if used > width {
            break;
        }
        display_char(character, &mut output);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::TextInput;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
    }

    fn type_text(input: &mut TextInput, text: &str) {
        for character in text.chars() {
            input.handle_key(key(KeyCode::Char(character)));
        }
    }

    #[test]
    fn multiline_input_splits_and_merges_lines() {
        let mut input = TextInput::multiline("abc");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Enter));
        input.handle_key(key(KeyCode::Char('x')));

        assert_eq!(input.value(), "ab\nxc");

        input.handle_key(key(KeyCode::Backspace));
        input.handle_key(key(KeyCode::Backspace));

        assert_eq!(input.value(), "abc");
    }

    #[test]
    fn single_line_input_replaces_newlines() {
        let input = TextInput::single("one\ntwo");

        assert_eq!(input.value(), "one two");
    }

    #[test]
    fn readline_keys_edit_single_line() {
        let mut input = TextInput::single("http://api.test/users/42");

        input.handle_key(ctrl('w'));
        assert_eq!(input.value(), "http://api.test/users/");

        input.handle_key(ctrl('a'));
        input.handle_key(ctrl('k'));
        assert_eq!(input.value(), "");

        type_text(&mut input, "abc");
        input.handle_key(ctrl('u'));
        assert!(input.is_empty());
    }

    #[test]
    fn word_movement_stops_at_url_separators() {
        let mut input = TextInput::single("http://api.test/users");
        input.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT));
        assert_eq!(input.cursor().1, "http://api.test/".len());
        input.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT));
        assert_eq!(input.cursor().1, "http://api.".len());
        input.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));
        assert_eq!(input.cursor().1, "http://api.test".len());
    }

    #[test]
    fn paste_into_single_line_drops_trailing_newline() {
        let mut input = TextInput::single("");
        input.insert_str("http://api.test/users\r\n");
        assert_eq!(input.value(), "http://api.test/users");
    }

    #[test]
    fn paste_into_multiline_keeps_lines_and_cursor() {
        let mut input = TextInput::multiline("");
        input.insert_str("{\n  \"a\": 1\n}");
        assert_eq!(input.value(), "{\n  \"a\": 1\n}");
        assert_eq!(input.cursor(), (2, 1));
    }

    #[test]
    fn enter_after_open_brace_indents_and_closes_block() {
        let mut input = TextInput::multiline("{}");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Enter));
        type_text(&mut input, "\"a\": 1");

        assert_eq!(input.value(), "{\n  \"a\": 1\n}");
    }

    #[test]
    fn closing_bracket_dedents_blank_line() {
        let mut input = TextInput::multiline("{");
        input.handle_key(key(KeyCode::Enter));
        input.handle_key(key(KeyCode::Char('}')));

        assert_eq!(input.value(), "{\n}");
    }

    #[test]
    fn multibyte_characters_are_edited_by_character() {
        let mut input = TextInput::single("héllo");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.value(), "hélo");
    }

    #[test]
    fn loaded_text_round_trips_exactly() {
        for text in ["a\tb\r\nc\r\n", "{\n\t\"a\": 1\n}", ""] {
            assert_eq!(TextInput::multiline(text).value(), text);
        }
    }

    #[test]
    fn tabs_are_drawn_as_spaces_and_counted_for_the_cursor() {
        let input = TextInput::multiline("\tx");
        assert_eq!(super::slice_by_width(&input.lines()[0], 0, 10), "    x");
        assert_eq!(super::width_of(input.lines()[0].chars()), 5);
    }

    #[test]
    fn single_line_view_scrolls_to_keep_cursor_visible() {
        let input = TextInput::single("abcdefghijklmnopqrstuvwxyz");
        let (visible, cursor_x) = input.single_line_view(10);

        assert!(visible.ends_with('z'));
        assert_eq!(cursor_x, visible.chars().count());
        assert!(cursor_x < 10);
    }
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInput {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    multiline: bool,
}

impl TextInput {
    pub fn single(value: impl Into<String>) -> TextInput {
        let mut input = TextInput {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            multiline: false,
        };
        input.set_value(value.into());
        input
    }

    pub fn multiline(value: impl Into<String>) -> TextInput {
        let mut input = TextInput {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            multiline: true,
        };
        input.set_value(value.into());
        input
    }

    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_value(&mut self, value: String) {
        self.lines = if self.multiline {
            value.split('\n').map(String::from).collect()
        } else {
            vec![value.replace('\n', " ")]
        };

        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.line_len(self.cursor_row);
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.insert_char(character);
                true
            }
            KeyCode::Backspace => {
                self.backspace();
                true
            }
            KeyCode::Delete => {
                self.delete();
                true
            }
            KeyCode::Enter if self.multiline => {
                self.insert_newline();
                true
            }
            KeyCode::Left => {
                self.move_left();
                true
            }
            KeyCode::Right => {
                self.move_right();
                true
            }
            KeyCode::Up if self.multiline => {
                self.move_up();
                true
            }
            KeyCode::Down if self.multiline => {
                self.move_down();
                true
            }
            KeyCode::Home => {
                self.cursor_col = 0;
                true
            }
            KeyCode::End => {
                self.cursor_col = self.line_len(self.cursor_row);
                true
            }
            _ => false,
        }
    }

    fn insert_char(&mut self, character: char) {
        let row = self.cursor_row;
        let mut chars = self.lines[row].chars().collect::<Vec<_>>();
        let col = self.cursor_col.min(chars.len());
        chars.insert(col, character);
        self.lines[row] = chars.into_iter().collect();
        self.cursor_col = col + 1;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let row = self.cursor_row;
            let mut chars = self.lines[row].chars().collect::<Vec<_>>();
            let remove_at = self.cursor_col - 1;
            if remove_at < chars.len() {
                chars.remove(remove_at);
                self.lines[row] = chars.into_iter().collect();
                self.cursor_col -= 1;
            }
            return;
        }

        if self.multiline && self.cursor_row > 0 {
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.line_len(self.cursor_row);
            self.lines[self.cursor_row].push_str(&current);
        }
    }

    fn delete(&mut self) {
        let row = self.cursor_row;
        let mut chars = self.lines[row].chars().collect::<Vec<_>>();
        if self.cursor_col < chars.len() {
            chars.remove(self.cursor_col);
            self.lines[row] = chars.into_iter().collect();
            return;
        }

        if self.multiline && self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
        }
    }

    fn insert_newline(&mut self) {
        let row = self.cursor_row;
        let chars = self.lines[row].chars().collect::<Vec<_>>();
        let split_at = self.cursor_col.min(chars.len());
        let left = chars[..split_at].iter().collect::<String>();
        let right = chars[split_at..].iter().collect::<String>();
        self.lines[row] = left;
        self.lines.insert(row + 1, right);
        self.cursor_row += 1;
        self.cursor_col = 0;
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
        let line_len = self.line_len(self.cursor_row);
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.multiline && self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row == 0 {
            return;
        }
        self.cursor_row -= 1;
        self.cursor_col = self.cursor_col.min(self.line_len(self.cursor_row));
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 >= self.lines.len() {
            return;
        }
        self.cursor_row += 1;
        self.cursor_col = self.cursor_col.min(self.line_len(self.cursor_row));
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

#[cfg(test)]
mod tests {
    use super::TextInput;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
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
}

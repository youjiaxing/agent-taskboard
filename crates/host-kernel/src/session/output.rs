pub(super) fn bounded_size(cols: u16, rows: u16) -> (u16, u16) {
    // The same bounds apply to the OS PTY and its in-memory screen projection.
    (cols.clamp(2, 512), rows.clamp(2, 256))
}

pub(super) struct TerminalOutput {
    parser: vt100::Parser,
    exited_alternate: Option<(String, String)>,
}

impl TerminalOutput {
    pub(super) fn new(cols: u16, rows: u16) -> Self {
        let (cols, rows) = bounded_size(cols, rows);
        Self {
            parser: vt100::Parser::new(rows, cols, 0),
            exited_alternate: None,
        }
    }

    pub(super) fn process(&mut self, bytes: &[u8]) {
        // vt100 leaves the alternate screen on a CSI ... l or ESC c. Split at
        // those possible terminators so even a complete TUI exit in one read
        // retains the answer before the parser restores the old primary screen.
        // The parser, not this byte filter, decides whether a switch occurred.
        for part in bytes.split_inclusive(|byte| matches!(byte, b'l' | b'c')) {
            let (last, prefix) = part.split_last().expect("nonempty PTY segment");
            self.parser.process(prefix);
            let previous = (matches!(last, b'l' | b'c') && self.parser.screen().alternate_screen())
                .then(|| self.parser.screen().contents());
            self.parser.process(std::slice::from_ref(last));
            if let Some(previous) = previous.filter(|text| !text.trim().is_empty()) {
                if !self.parser.screen().alternate_screen() {
                    self.exited_alternate = Some((previous, self.parser.screen().contents()));
                }
            }
        }
    }

    pub(super) fn resize(&mut self, cols: u16, rows: u16) {
        let (cols, rows) = bounded_size(cols, rows);
        self.parser.screen_mut().set_size(rows, cols);
    }

    pub(super) fn contents(&self) -> String {
        let current = self.parser.screen().contents();
        if !self.parser.screen().alternate_screen() {
            if let Some((answer, restored_primary)) = &self.exited_alternate {
                return if current == *restored_primary || current.trim().is_empty() {
                    answer.clone()
                } else {
                    format!("{answer}\n{current}")
                };
            }
        }
        current
    }
}

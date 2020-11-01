// The example only works on Linux since Termion does not yet support
// Windows: https://gitlab.redox-os.org/redox-os/termion/-/issues/103
// The precise library doesn't matter much, so feel free to send a PR
// if there is a library with good Windows support.

fn main() -> Result<(), std::io::Error> {
    #[cfg(not(unix))]
    panic!("Sorry, this example currently only works on Unix!");

    #[cfg(unix)]
    unix_only::main()
}

#[cfg(unix)]
mod unix_only {
    use std::io::{self, Write};
    use termion::event::Key;
    use termion::input::TermRead;
    use termion::raw::{IntoRawMode, RawTerminal};
    use termion::screen::AlternateScreen;
    use termion::{color, cursor, style};
    use textwrap::{wrap, HyphenSplitter, NoHyphenation, WordSplitter, Wrapper};

    #[cfg(feature = "hyphenation")]
    use hyphenation::{Language, Load, Standard};

    fn draw_margins(
        row: u16,
        col: u16,
        line_width: u16,
        left: char,
        right: char,
        stdout: &mut RawTerminal<io::Stdout>,
    ) -> Result<(), io::Error> {
        write!(
            stdout,
            "{}{}{}{}",
            cursor::Goto(col - 1, row),
            color::Fg(color::Red),
            left,
            color::Fg(color::Reset),
        )?;
        write!(
            stdout,
            "{}{}{}{}",
            cursor::Goto(col + line_width, row),
            color::Fg(color::Red),
            right,
            color::Fg(color::Reset),
        )?;

        Ok(())
    }

    fn draw_text<'a>(
        text: &str,
        options: &Wrapper<'a, dyn WordSplitter>,
        splitter_label: &str,
        stdout: &mut RawTerminal<io::Stdout>,
    ) -> Result<(), io::Error> {
        let mut row: u16 = 1;
        let col: u16 = 3;

        write!(stdout, "{}", termion::clear::All)?;
        write!(
            stdout,
            "{}{}Settings:{}",
            cursor::Goto(col, row),
            style::Bold,
            style::Reset,
        )?;
        row += 1;

        write!(
            stdout,
            "{}- width: {}{}{} (use ← and → to change)",
            cursor::Goto(col, row),
            style::Bold,
            options.width,
            style::Reset,
        )?;
        row += 1;

        write!(
            stdout,
            "{}- break_words: {}{:?}{} (toggle with Ctrl-b)",
            cursor::Goto(col, row),
            style::Bold,
            options.break_words,
            style::Reset,
        )?;
        row += 1;

        write!(
            stdout,
            "{}- splitter: {}{}{} (cycle with Ctrl-s)",
            cursor::Goto(col, row),
            style::Bold,
            splitter_label,
            style::Reset,
        )?;
        row += 2;

        let mut lines = options.wrap(text);
        if let Some(line) = lines.last() {
            // If `text` ends with a newline, the final wrapped line
            // contains this newline. This will in turn leave the
            // cursor hanging in the middle of the line. Pushing an
            // extra empty line fixes this.
            if line.ends_with('\n') {
                lines.push("".into());
            }
        } else {
            // No lines -> we add an empty line so we have a place
            // where we can display the cursor.
            lines.push("".into());
        }

        // Draw margins extended one line above and below the wrapped
        // text. This serves to indicate the margins if `break_words`
        // is `false` and `width` is very small.
        draw_margins(row, col, options.width as u16, '┌', '┐', stdout)?;
        let final_row = row + lines.len() as u16 + 1;
        draw_margins(final_row, col, options.width as u16, '└', '┘', stdout)?;
        row += 1;

        for line in lines {
            draw_margins(row, col, options.width as u16, '│', '│', stdout)?;
            write!(stdout, "{}{}", cursor::Goto(col, row), line)?;
            row += 1;
        }

        stdout.flush()
    }

    pub fn main() -> Result<(), io::Error> {
        let initial_width = 20;

        type SplitterChanger = Box<
            dyn for<'a> Fn(&'_ Wrapper<'a, dyn WordSplitter>) -> Box<Wrapper<'a, dyn WordSplitter>>,
        >;

        let mut labels = vec![
            String::from("HyphenSplitter"),
            String::from("NoHyphenation"),
        ];

        let mut splitters: Vec<SplitterChanger> = vec![
            Box::new(|w| Box::new(w.splitter(HyphenSplitter))),
            Box::new(|w| Box::new(w.splitter(NoHyphenation))),
        ];

        // If you like, you can download more dictionaries from
        // https://github.com/tapeinosyne/hyphenation/tree/master/dictionaries
        // Place the dictionaries in the examples/ directory. Here we
        // just load the embedded en-us dictionary.
        #[cfg(feature = "hyphenation")]
        for lang in &[Language::EnglishUS] {
            let dictionary = Standard::from_embedded(*lang).or_else(|_| {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("examples")
                    .join(format!("{}.standard.bincode", lang.code()));
                Standard::from_path(*lang, &path)
            });

            if let Ok(dict) = dictionary {
                labels.push(format!("{} hyphenation", lang.code()));
                splitters.push(Box::new(move |w| Box::new(w.splitter(dict.clone()))));
            }
        }

        let mut idx_iter = (0..splitters.len()).collect::<Vec<_>>().into_iter().cycle();

        let (mut label, mut options) = {
            let idx = idx_iter.next().unwrap();

            let label = labels[idx].clone();
            let mut options: Box<Wrapper<dyn WordSplitter>> =
                Box::new(Wrapper::new(initial_width).break_words(false));
            options = splitters[idx](&options);

            (label, options)
        };

        let mut text = String::from(
            "Welcome to the interactive word-wrapping demo! Use the arrow \
        keys to change the line length and try typing your own text!",
        );

        let stdin = io::stdin();
        let mut screen = AlternateScreen::from(io::stdout().into_raw_mode()?);
        write!(screen, "{}", cursor::BlinkingUnderline)?;
        draw_text(&text, &options, &label, &mut screen)?;

        for c in stdin.keys() {
            match c? {
                Key::Esc | Key::Ctrl('c') => break,
                Key::Left => options.width = options.width.saturating_sub(1),
                Key::Right => options.width = options.width.saturating_add(1),
                Key::Ctrl('b') => options.break_words = !options.break_words,
                Key::Ctrl('s') => {
                    let idx = idx_iter.next().unwrap();
                    options = splitters[idx](&options);
                    label = labels[idx].clone();
                }
                Key::Char(c) => text.push(c),
                Key::Backspace => {
                    text.pop();
                }
                _ => {}
            }

            draw_text(&text, &options, &label, &mut screen)?;
        }

        // TODO: change to cursor::DefaultStyle if
        // https://github.com/redox-os/termion/pull/157 is merged.
        screen.write(b"\x1b[0 q")?;
        screen.flush()
    }
}

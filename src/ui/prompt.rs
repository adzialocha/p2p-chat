use std::io::{self, Write};
use std::str::FromStr;

use termion::clear::CurrentLine as ClearLine;
use termion::cursor::Goto;
use termion::event::{Event, Key};

#[derive(Default)]
pub struct Prompt {
    dex: usize,
    chars: String,
}

impl Prompt {
    pub fn handle_input(&mut self, input: &Event) -> Result<Option<String>, io::Error> {
        match input {
            Event::Key(Key::Char('\n')) => self.finalize(),
            Event::Key(Key::Backspace) => Ok(self.back()),
            Event::Key(Key::Delete) => Ok(self.delete()),
            Event::Key(Key::Left) => Ok(self.left()),
            Event::Key(Key::Right) => Ok(self.right()),
            Event::Key(Key::Char(chr)) => Ok(self.new_key(*chr)),
            _ => Ok(None),
        }
    }

    fn left(&mut self) -> Option<String> {
        if self.dex > 0 {
            self.dex -= 1;
        }
        None
    }

    fn right(&mut self) -> Option<String> {
        if self.dex < self.chars.len() {
            self.dex += 1;
        }
        None
    }

    fn delete(&mut self) -> Option<String> {
        if self.dex < self.chars.len() {
            self.chars.remove(self.dex);
        }
        None
    }

    fn back(&mut self) -> Option<String> {
        if !self.chars.is_empty() {
            self.dex -= 1;
            self.chars.remove(self.dex);
        }
        None
    }

    fn new_key(&mut self, chr: char) -> Option<String> {
        self.chars.insert(self.dex, chr);
        self.dex += 1;
        None
    }

    fn finalize(&mut self) -> Result<Option<String>, io::Error> {
        if self.chars.is_empty() {
            return Ok(None);
        }

        let message = FromStr::from_str(&self.chars).unwrap();

        self.chars.drain(..);
        self.dex = 0;

        Ok(Some(message))
    }

    pub fn render<W: Write>(&mut self, w: &mut W, row: u16) -> Result<(), io::Error> {
        if let Err(err) = write!(
            w,
            "{}{}:{}{}",
            Goto(1, row),
            ClearLine,
            self.chars,
            Goto(self.dex as u16 + 2, row)
        ) {
            panic!("Failed to render command prompt: {:?}", err);
        }
        Ok(())
    }
}

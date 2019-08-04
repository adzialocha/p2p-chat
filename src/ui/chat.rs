use std::cmp;
use std::io::{self, Write};

use chrono::{DateTime, Local};
use termion::clear::CurrentLine as ClearLine;
use termion::cursor::Goto;

const DEFAULT_SENDER: &str = "INFO";

pub struct ChatMessage {
    sender: Option<String>,
    text: String,
    timestamp: DateTime<Local>,
}

impl ChatMessage {
    pub fn new(sender: String, text: String) -> Self {
        Self {
            sender: Some(sender),
            text,
            timestamp: Local::now(),
        }
    }

    pub fn from_string(text: String) -> Self {
        Self {
            sender: None,
            text,
            timestamp: Local::now(),
        }
    }

    pub fn render(&self, max_len: usize) -> String {
        let mut line = format!("[{}] {}: {}",
                               self.timestamp.format("%H:%M:%S").to_string(),
                               self.sender.clone().unwrap_or(String::from(DEFAULT_SENDER)),
                               self.text);

        // Truncate line when it exceeds our window width
        if line.len() > max_len as usize {
            line.truncate(max_len as usize - 3);
            line.push_str("...");
        }

        line
    }
}

#[derive(Default)]
pub struct Chat {
    messages: Vec<ChatMessage>,
}

impl Chat {
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn render<W: Write>(
        &mut self,
        w: &mut W,
        rows: u16,
        columns: u16,
    )
        -> Result<(), io::Error>
    {
        let start = cmp::max(0, self.messages.len() as isize - (rows - 1) as isize);
        let size = rows;

        let lines = self
            .messages
            .iter()
            .skip(start as usize)
            .take(size as usize);

        let mut line_strings = String::new();
        for (line_index, message) in lines.enumerate() {
            let rendered_line = format!("{}{}{}",
                                        Goto(0, line_index as u16 + 1),
                                        ClearLine,
                                        &message.render(columns as usize));

            line_strings.push_str(&rendered_line);
        }

        w.write_all(line_strings.as_bytes())?;

        Ok(())
    }
}

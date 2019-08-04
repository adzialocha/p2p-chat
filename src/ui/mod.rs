mod chat;
mod prompt;
mod terminal;

use std::io::{self, Write};

use futures::sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::{Async, Poll, Stream};
use termion::event::{Event, Key};

pub use chat::ChatMessage;
use chat::Chat;
use prompt::Prompt;
use terminal::{Terminal, TerminalEvent};

pub struct UserInterface {
    // Chat interface to display recent ChatMessages
    chat: Chat,

    // Did user send exit command?
    exit: bool,

    // Buffer to store user input from prompt
    input: Option<String>,

    // Incoming messages to display
    messages_rx: UnboundedReceiver<ChatMessage>,

    // User input prompt interface
    prompt: Prompt,

    // Current size of the Terminal (columns, rows)
    term_size: (u16, u16),

    // Our terminal instance to write to
    terminal: Terminal,
}

impl UserInterface {
    pub fn new() -> Result<(Self, UnboundedSender<ChatMessage>), io::Error> {
        let (messages_tx, messages_rx) = unbounded();

        let view = Self {
            chat: Chat::default(),
            exit: false,
            input: None,
            messages_rx,
            prompt: Prompt::default(),
            term_size: (0, 0),
            terminal: Terminal::new()?,
        };

        Ok((view, messages_tx))
    }

    fn handle_resize(&mut self, size: (u16, u16)) {
        self.term_size = size;
    }

    fn handle_input(&mut self, event: Event) {
        match event {
            // Received a signal to exit application
            Event::Key(Key::Ctrl('c')) => self.exit = true,

            // Normal key input, give it to prompt
            event => {
                match self.prompt.handle_input(&event) {
                    Ok(None) => {
                    },
                    Ok(Some(input)) => {
                        self.input = Some(input)
                    },
                    Err(err) => {
                        panic!("Failed to parse command: {:?}", err);
                    }
                }
            }
        }
    }

    fn render(&mut self) -> Result<(), io::Error> {
        // Render interface components
        self.chat.render(self.terminal.stdout(), self.term_size.1, self.term_size.0)?;
        self.prompt.render(self.terminal.stdout(), self.term_size.1)?;

        if let Err(e) = self.terminal.stdout().flush() {
            panic!("failed to flush stdout: {}", e);
        }

        Ok(())
    }

    fn poll_messages(&mut self) {
        // Check for incoming messages and give them to Chat interface
        match self.messages_rx.poll() {
            Ok(Async::Ready(Some(message))) => {
                self.chat.add_message(message);
                return;
            }
            _ => return,
        }
    }

    fn poll_terminal(&mut self) {
        // Check for input and resize events of the Terminal
        loop {
            match self.terminal.poll() {
                Ok(Async::Ready(Some(event))) => match event {
                    TerminalEvent::Input(event) => self.handle_input(event),
                    TerminalEvent::Resize(event) => self.handle_resize(event),
                },
                Ok(Async::Ready(None)) => {
                    self.exit = true;
                    return;
                }
                Ok(Async::NotReady) => {
                    return;
                }
                Err(_) => {
                    self.exit = true;
                    return;
                }
            }
        }
    }
}

impl Stream for UserInterface {
    type Item = String;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // Check for interface changes
        self.poll_terminal();
        self.poll_messages();

        // End stream when user indicated exit
        if self.exit {
            return Ok(Async::Ready(None));
        }

        // Render to the view
        self.render().expect("failed to render the view");

        // UserInterface is a Stream returning input Strings from the prompt
        match &self.input {
            Some(input) => {
                let message_clone = input.clone();
                self.input = None;

                Ok(Async::Ready(Some(message_clone)))
            },
            None => Ok(Async::NotReady),
        }
    }
}

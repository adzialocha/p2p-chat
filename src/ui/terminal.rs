use std::io::{self, Stdout};
use std::thread;
use std::time::Duration;

use futures::sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::{Async, Poll, Sink, Stream};
use termion::event::Event;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use termion::terminal_size;

type RenderTarget = AlternateScreen<RawTerminal<Stdout>>;

pub struct Terminal {
    size: UnboundedReceiver<(u16, u16)>,
    stdin: UnboundedReceiver<Event>,
    stdout: RenderTarget,
}

impl Terminal {
    pub fn new() -> Result<Self, io::Error> {
        let (stdin_tx, stdin_rx) = unbounded();
        let (size_tx, size_rx) = unbounded();

        let stdout = AlternateScreen::from(
            io::stdout()
                .into_raw_mode()
                .expect("Failed to put terminal into raw mode"),
        );

        let term = Terminal {
            stdin: stdin_rx,
            size: size_rx,
            stdout,
        };

        Terminal::start_stdin_listening(stdin_tx);
        Terminal::start_size_listening(size_tx);

        Ok(term)
    }

    fn start_size_listening(tx: UnboundedSender<(u16, u16)>) {
        let mut tx = tx;
        thread::spawn(move || {
            let mut current_size = (0, 0);
            loop {
                match terminal_size() {
                    Ok(new_size) => {
                        if new_size != current_size {
                            current_size = new_size;
                            let _ = tx.start_send(current_size).unwrap();
                            let _ = tx.poll_complete().unwrap();
                        }
                    }
                    Err(e) => {
                        panic!("failed to get terminal size: {}", e);
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    fn start_stdin_listening(tx: UnboundedSender<Event>) {
        let mut tx = tx;
        thread::spawn(move || {
            for event_res in io::stdin().events() {
                match event_res {
                    Ok(event) => {
                        let _ = tx.start_send(event).unwrap();
                        let _ = tx.poll_complete().unwrap();
                    }
                    Err(e) => panic!("{}", e),
                }
            }
        });
    }

    pub fn stdout(&mut self) -> &mut RenderTarget {
        &mut self.stdout
    }
}

pub enum TerminalEvent {
    Resize((u16, u16)),
    Input(Event),
}

impl Stream for Terminal {
    type Item = TerminalEvent;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.size.poll() {
            Ok(Async::Ready(Some(size))) => {
                let event = TerminalEvent::Resize(size);
                return Ok(Async::Ready(Some(event)));
            }
            Ok(Async::Ready(None)) => {
                return Ok(Async::Ready(None));
            }
            Ok(Async::NotReady) => {}
            Err(()) => return Err(()),
        }

        match self.stdin.poll() {
            Ok(Async::Ready(Some(event))) => {
                let event = TerminalEvent::Input(event);
                return Ok(Async::Ready(Some(event)));
            }
            Ok(Async::Ready(None)) => {
                return Ok(Async::Ready(None));
            }
            Ok(Async::NotReady) => {}
            Err(()) => return Err(()),
        }

        Ok(Async::NotReady)
    }
}

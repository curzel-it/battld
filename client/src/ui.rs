use std::io::{self, Write};
use crossterm::{event::{self, Event}, terminal};

pub fn clear_screen() -> io::Result<()> {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush()?;
    Ok(())
}

pub fn drain_stdin_buffer() {
    // Use crossterm to drain any buffered input
    let _ = terminal::enable_raw_mode();

    // Drain all pending events
    while let Ok(true) = event::poll(std::time::Duration::from_millis(0)) {
        let _ = event::read();
    }

    let _ = terminal::disable_raw_mode();
}

pub fn wait_for_keypress() -> io::Result<()> {
    // First, drain any pending events in the terminal buffer
    terminal::enable_raw_mode()?;

    // Clear any buffered input
    while event::poll(std::time::Duration::from_millis(10))? {
        event::read()?; // Consume and discard
    }

    // Now wait for an actual keypress
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(_) = event::read()? {
                break;
            }
        }
    }

    terminal::disable_raw_mode()?;
    Ok(())
}

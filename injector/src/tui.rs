use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::Print,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;

pub fn run_tui() {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide).unwrap();

    println!("DarkClient Injector (TUI)");
    println!("Press 'f' to find the PID, 'i' to inject, 'q' to quit.");

    let mut pid: Option<u32> = None;
    let mut status = String::from("Ready.");

    loop {
        println!("Status: {}", status);
        if event::poll(std::time::Duration::from_millis(500)).unwrap() {
            if let Event::Key(key_event) = event::read().unwrap() {
                match key_event.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('f') => {
                        pid = super::platform::find_pid();
                        status = if let Some(p) = pid {
                            format!("PID found: {}", p)
                        } else {
                            "PID not found.".to_string()
                        };
                    }
                    KeyCode::Char('i') => {
                        if let Some(p) = pid {
                            match super::platform::inject(p) {
                                Ok(_) => status = "Injection successful!".to_string(),
                                Err(e) => status = format!("Injection error: {}", e),
                            }
                        } else {
                            status = "Find the PID first.".to_string();
                        }
                    }
                    _ => {}
                }
            }
        }
        // Clear the line to update the status
        execute!(
            stdout,
            cursor::MoveTo(0, 3),
            Print(" ".repeat(50)),
            cursor::MoveTo(0, 3)
        )
        .unwrap();
    }

    execute!(stdout, LeaveAlternateScreen, cursor::Show).unwrap();
    println!("Exited TUI.");
}

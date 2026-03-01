use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use opentui_rust::{Renderer, Rgba, Style};
use std::io::stdout;

fn main() -> std::io::Result<()> {
    let mut renderer = Renderer::new(80, 24)?;
    let mut running = true;

    stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    while running {
        let buffer = renderer.buffer();
        buffer.clear(Rgba::BLACK);
        buffer.draw_text(10, 5, "Hello, OpenTUI!", Style::fg(Rgba::GREEN));
        buffer.draw_text(10, 8, "Press Ctrl+C to quit", Style::fg(Rgba::WHITE));
        renderer.present()?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('c')
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            running = false
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}

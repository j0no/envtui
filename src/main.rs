use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use opentui_rust::{Renderer, Rgba, Style};
use std::env::vars_os;
use std::io::stdout;

const CYAN: Rgba = Rgba::new(0.0, 1.0, 1.0, 1.0);
const GRAY: Rgba = Rgba::new(0.5, 0.5, 0.5, 1.0);

fn main() -> std::io::Result<()> {
    let mut renderer = Renderer::new(80, 24)?;
    let mut running = true;
    let mut scroll_offset: usize = 0;

    let env_vars: Vec<(String, String)> = vars_os()
        .filter_map(|(k, v)| {
            let key = k.into_string().ok()?;
            let val = v.into_string().ok()?;
            Some((key, val))
        })
        .collect();

    stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    while running {
        let buffer = renderer.buffer();
        buffer.clear(Rgba::BLACK);

        buffer.draw_text(
            1,
            0,
            "Environment Variables",
            Style::fg(CYAN).merge(Style::bold()),
        );

        let visible_rows = 20;
        let total_lines = env_vars.len() * 2;

        for (i, (key, val)) in env_vars
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_rows)
        {
            let line_idx = i - scroll_offset;
            let y_key: u32 = (2 + line_idx * 2) as u32;
            let y_val: u32 = (3 + line_idx * 2) as u32;

            if y_val >= 24 {
                break;
            }

            let key_display = if key.len() > 38 {
                format!("{}...", &key[..35])
            } else {
                key.clone()
            };
            let val_display = if val.len() > 76 {
                format!("{}...", &val[..73])
            } else {
                val.clone()
            };

            buffer.draw_text(1, y_key, &key_display, Style::fg(Rgba::GREEN));
            buffer.draw_text(1, y_val, &val_display, Style::fg(Rgba::WHITE));
        }

        let scroll_info = format!(
            "Scroll: {}/{} (arrows to move, Ctrl+C to quit)",
            scroll_offset / 2 + 1,
            (total_lines + 1) / 2
        );
        buffer.draw_text(1, 23, &scroll_info, Style::fg(GRAY));

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
                        KeyCode::Up => {
                            if scroll_offset >= 2 {
                                scroll_offset -= 2;
                            }
                        }
                        KeyCode::Down => {
                            if scroll_offset + 40 < total_lines {
                                scroll_offset += 2;
                            }
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

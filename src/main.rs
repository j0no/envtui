use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use opentui_rust::{terminal_size, Renderer, Rgba, Style};
use std::env::vars_os;
use std::io::stdout;

const CYAN: Rgba = Rgba::new(0.0, 1.0, 1.0, 1.0);
const GRAY: Rgba = Rgba::new(0.5, 0.5, 0.5, 1.0);

fn get_terminal_size() -> (u32, u32) {
    terminal_size()
        .map(|(w, h)| (w as u32, h as u32))
        .unwrap_or((80, 24))
}

fn main() -> std::io::Result<()> {
    let (width, height) = get_terminal_size();
    let mut renderer = Renderer::new(width, height)?;
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
        let (term_width, term_height) = get_terminal_size();
        let buf_width = renderer.buffer().size().0;
        let buf_height = renderer.buffer().size().1;

        if term_width != buf_width || term_height != buf_height {
            renderer = Renderer::new(term_width, term_height)?;
        }

        let buffer = renderer.buffer();
        let width = term_width as usize;
        let height = term_height as usize;

        buffer.clear(Rgba::BLACK);

        buffer.draw_text(
            1,
            0,
            "Environment Variables",
            Style::fg(CYAN).merge(Style::bold()),
        );

        let separator = "=".repeat(width.saturating_sub(2));
        buffer.draw_text(1, 1, &separator, Style::fg(GRAY));

        let key_max_len = width.saturating_sub(4);
        let val_max_len = width.saturating_sub(4);

        let visible_rows = height.saturating_sub(4);
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

            if y_val as usize >= height {
                break;
            }

            let key_display = if key.len() > key_max_len {
                format!("{}...", &key[..key_max_len.saturating_sub(3)])
            } else {
                key.clone()
            };
            let val_display = if val.len() > val_max_len {
                format!("{}...", &val[..val_max_len.saturating_sub(3)])
            } else {
                val.clone()
            };

            buffer.draw_text(1, y_key, &key_display, Style::fg(Rgba::GREEN));
            buffer.draw_text(1, y_val, &val_display, Style::fg(Rgba::WHITE));
        }

        let scroll_info = format!(
            "Scroll: {}/{} (arrows to move, Ctrl+C to quit) | {} vars",
            scroll_offset / 2 + 1,
            (total_lines + 1) / 2,
            env_vars.len()
        );
        let footer_y = (height - 1) as u32;
        buffer.draw_text(1, footer_y, &scroll_info, Style::fg(GRAY));

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
                            if scroll_offset + (visible_rows * 2) < total_lines {
                                scroll_offset += 2;
                            }
                        }
                        KeyCode::PageUp => {
                            scroll_offset = scroll_offset.saturating_sub(visible_rows * 2);
                        }
                        KeyCode::PageDown => {
                            scroll_offset = (scroll_offset + visible_rows * 2)
                                .min(total_lines.saturating_sub(1));
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

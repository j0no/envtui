use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use opentui_rust::{terminal_size, Renderer, Rgba, Style};
use std::collections::HashMap;
use std::env::vars_os;
use std::fs;
use std::io::stdout;
use std::path::PathBuf;

const CYAN: Rgba = Rgba::new(0.0, 1.0, 1.0, 1.0);
const GRAY: Rgba = Rgba::new(0.5, 0.5, 0.5, 1.0);
const DARK_GRAY: Rgba = Rgba::new(0.2, 0.2, 0.2, 1.0);
const SELECTED_BG: Rgba = Rgba::new(0.3, 0.3, 0.5, 1.0);
const YELLOW: Rgba = Rgba::new(1.0, 1.0, 0.0, 1.0);

enum SidebarItem {
    File(PathBuf),
    SystemEnv,
}

fn get_terminal_size() -> (u32, u32) {
    terminal_size()
        .map(|(w, h)| (w as u32, h as u32))
        .unwrap_or((80, 24))
}

fn get_sidebar_items() -> Vec<SidebarItem> {
    let mut items: Vec<SidebarItem> = fs::read_dir(".")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(".env") || n.ends_with(".env"))
                        .unwrap_or(false)
                })
                .map(SidebarItem::File)
                .collect()
        })
        .unwrap_or_default();

    items.sort_by(|a, b| match (a, b) {
        (SidebarItem::File(path_a), SidebarItem::File(path_b)) => {
            let name_a = path_a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let name_b = path_b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name_a.cmp(name_b)
        }
        _ => std::cmp::Ordering::Equal,
    });

    items.push(SidebarItem::SystemEnv);
    items
}

fn parse_env_file(path: &PathBuf) -> HashMap<String, String> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut vars = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_string();
            let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
            vars.insert(key, val);
        }
    }

    vars
}

fn main() -> std::io::Result<()> {
    let (width, height) = get_terminal_size();
    let mut renderer = Renderer::new(width, height)?;
    let mut running = true;
    let mut scroll_offset: usize = 0;
    let mut sidebar_scroll: usize = 0;
    let mut selected_idx: usize = 0;
    let mut focused_panel: usize = 0;

    let sidebar_items = get_sidebar_items();
    let env_vars: Vec<(String, String)> = vars_os()
        .filter_map(|(k, v)| {
            let key = k.into_string().ok()?;
            let val = v.into_string().ok()?;
            Some((key, val))
        })
        .collect();

    let mut current_content: HashMap<String, String> = HashMap::new();
    if let Some(item) = sidebar_items.get(selected_idx) {
        match item {
            SidebarItem::File(path) => {
                current_content = parse_env_file(path);
            }
            SidebarItem::SystemEnv => {
                current_content = env_vars.iter().cloned().collect();
            }
        }
    }

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

        let sidebar_width = 25.min(width / 3);
        let main_width = width.saturating_sub(sidebar_width);

        buffer.clear(Rgba::BLACK);

        for y in 0..height {
            for x in 0..sidebar_width {
                buffer.draw_text(x as u32, y as u32, " ", Style::bg(DARK_GRAY));
            }
        }

        let sidebar_visible = height.saturating_sub(2);
        let total_files = sidebar_items.len();

        let sidebar_title_style = if focused_panel == 0 {
            Style::fg(CYAN).merge(Style::bold()).merge(Style::inverse())
        } else {
            Style::fg(CYAN).merge(Style::bold())
        };
        buffer.draw_text(0, 0, "Select Source", sidebar_title_style);

        let sidebar_separator = if focused_panel == 0 {
            "█".repeat(sidebar_width.saturating_sub(1))
        } else {
            "─".repeat(sidebar_width.saturating_sub(1))
        };
        let sep_color = if focused_panel == 0 { CYAN } else { GRAY };
        buffer.draw_text(1, 1, &sidebar_separator, Style::fg(sep_color));

        let sidebar_focus_indicator = if focused_panel == 0 { "» " } else { "  " };
        buffer.draw_text(0, 2, sidebar_focus_indicator, Style::fg(CYAN));

        for (i, item) in sidebar_items
            .iter()
            .enumerate()
            .skip(sidebar_scroll)
            .take(sidebar_visible)
        {
            let line_idx = i - sidebar_scroll;
            let y = (2 + line_idx) as u32;

            let label = match item {
                SidebarItem::File(path) => path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                SidebarItem::SystemEnv => "<System Env>",
            };

            let is_selected = i == selected_idx;

            if is_selected {
                for x in 0..sidebar_width {
                    buffer.draw_text(x as u32, y, " ", Style::bg(SELECTED_BG));
                }
                buffer.draw_text(
                    1,
                    y,
                    label,
                    Style::fg(Rgba::WHITE).merge(Style::bg(SELECTED_BG)),
                );
            } else {
                buffer.draw_text(1, y, label, Style::fg(Rgba::GREEN));
            }
        }

        buffer.draw_text(
            sidebar_width as u32 + 1,
            0,
            "Variables",
            Style::fg(CYAN).merge(Style::bold()),
        );

        let _separator = "=".repeat(main_width.saturating_sub(2));
        let content_separator = if focused_panel == 1 {
            "█".repeat(main_width.saturating_sub(2))
        } else {
            "=".repeat(main_width.saturating_sub(2))
        };
        let content_sep_color = if focused_panel == 1 { CYAN } else { GRAY };
        buffer.draw_text(
            sidebar_width as u32 + 1,
            1,
            &content_separator,
            Style::fg(content_sep_color),
        );

        let title = match sidebar_items.get(selected_idx) {
            Some(SidebarItem::File(path)) => {
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            }
            Some(SidebarItem::SystemEnv) => "System Environment",
            None => "No source",
        };

        let content_title_style = if focused_panel == 1 {
            Style::fg(YELLOW).merge(Style::inverse())
        } else {
            Style::fg(YELLOW)
        };
        buffer.draw_text(sidebar_width as u32 + 1, 2, title, content_title_style);

        let content_focus_indicator = if focused_panel == 1 { "» " } else { "  " };
        buffer.draw_text(
            sidebar_width as u32,
            2,
            content_focus_indicator,
            Style::fg(CYAN),
        );

        let key_max_len = main_width.saturating_sub(4);
        let val_max_len = main_width.saturating_sub(4);

        let content_vars: Vec<(String, String)> = current_content
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let visible_rows = height.saturating_sub(5);

        for (i, (key, val)) in content_vars
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_rows)
        {
            let line_idx = i - scroll_offset;
            let y_key: u32 = (4 + line_idx * 2) as u32;
            let y_val: u32 = (5 + line_idx * 2) as u32;

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

            buffer.draw_text(
                sidebar_width as u32 + 1,
                y_key,
                &key_display,
                Style::fg(Rgba::GREEN),
            );
            buffer.draw_text(
                sidebar_width as u32 + 1,
                y_val,
                &val_display,
                Style::fg(Rgba::WHITE),
            );
        }

        let scroll_info = format!(
            "Scroll: {}/{} | Items: {} | {} selected",
            scroll_offset / 2 + 1,
            (content_vars.len() * 2 + 1) / 2,
            total_files,
            match sidebar_items.get(selected_idx) {
                Some(SidebarItem::File(path)) =>
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                Some(SidebarItem::SystemEnv) => "<System Env>",
                None => "None",
            }
        );
        let footer_y = (height - 1) as u32;
        buffer.draw_text(1, footer_y, &scroll_info, Style::fg(GRAY));

        let help = "(arrows: move, Ctrl+C: quit)";
        buffer.draw_text(
            (width.saturating_sub(help.len())) as u32,
            footer_y,
            help,
            Style::fg(GRAY),
        );

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
                        KeyCode::Tab => {
                            focused_panel = if focused_panel == 0 { 1 } else { 0 };
                        }
                        KeyCode::Up => {
                            if focused_panel == 0 {
                                if sidebar_scroll > 0 && selected_idx == sidebar_scroll {
                                    sidebar_scroll -= 1;
                                }
                                if selected_idx > 0 {
                                    selected_idx -= 1;
                                }
                            } else if scroll_offset >= 2 {
                                scroll_offset -= 2;
                            }
                        }
                        KeyCode::Down => {
                            if focused_panel == 0 {
                                if selected_idx + 1 < sidebar_items.len() {
                                    selected_idx += 1;
                                    if selected_idx >= sidebar_scroll + sidebar_visible {
                                        sidebar_scroll =
                                            selected_idx.saturating_sub(sidebar_visible - 1);
                                    }
                                    if let Some(item) = sidebar_items.get(selected_idx) {
                                        match item {
                                            SidebarItem::File(path) => {
                                                current_content = parse_env_file(path);
                                            }
                                            SidebarItem::SystemEnv => {
                                                current_content =
                                                    env_vars.iter().cloned().collect();
                                            }
                                        }
                                    }
                                }
                            } else if scroll_offset + 40 < content_vars.len() * 2 {
                                scroll_offset += 2;
                            }
                        }
                        KeyCode::PageUp => {
                            if focused_panel == 0 {
                                sidebar_scroll = sidebar_scroll.saturating_sub(sidebar_visible);
                                selected_idx = sidebar_scroll;
                            } else {
                                scroll_offset = scroll_offset.saturating_sub(height * 2);
                            }
                        }
                        KeyCode::PageDown => {
                            if focused_panel == 0 {
                                sidebar_scroll = (sidebar_scroll + sidebar_visible)
                                    .min(sidebar_items.len().saturating_sub(sidebar_visible));
                                selected_idx = sidebar_scroll;
                            } else {
                                scroll_offset = (scroll_offset + height * 2)
                                    .min((content_vars.len() * 2).saturating_sub(1));
                            }
                        }
                        KeyCode::Left => {
                            if focused_panel == 1 {
                                focused_panel = 0;
                            }
                        }
                        KeyCode::Right => {
                            if focused_panel == 0 && selected_idx + 1 < sidebar_items.len() {
                                selected_idx += 1;
                                scroll_offset = 0;
                                if selected_idx >= sidebar_scroll + sidebar_visible {
                                    sidebar_scroll =
                                        selected_idx.saturating_sub(sidebar_visible - 1);
                                }
                                if let Some(item) = sidebar_items.get(selected_idx) {
                                    match item {
                                        SidebarItem::File(path) => {
                                            current_content = parse_env_file(path);
                                        }
                                        SidebarItem::SystemEnv => {
                                            current_content = env_vars.iter().cloned().collect();
                                        }
                                    }
                                }
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

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    ExecutableCommand,
};
use opentui_rust::{buffer::BoxStyle, terminal_size, Renderer, Rgba, Style};
mod colors;
use colors::{CYAN, DODGER_BLUE, GRAY, SELECTED_BG, YELLOW};
use std::collections::HashMap;
use std::env::vars_os;
use std::fs;
use std::io::stdout;
use std::path::PathBuf;

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
    // Get items from current and filter for env files
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
    // Sort Alphabetical
    items.sort_by(|a, b| match (a, b) {
        (SidebarItem::File(path_a), SidebarItem::File(path_b)) => {
            let name_a = path_a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let name_b = path_b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name_a.cmp(name_b)
        }
        _ => std::cmp::Ordering::Equal,
    });
    // Added SystemEnv type
    items.push(SidebarItem::SystemEnv);
    items
}

// Get env vars
fn parse_env_file(path: &PathBuf) -> Vec<(String, String)> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut vars: Vec<(String, String)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_string();
            let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() {
                vars.push((key, val));
            }
        }
    }

    vars
}

fn switch_to_item(
    idx: usize,
    sidebar_items: &[SidebarItem],
    env_vars: &[(String, String)],
    scroll_offsets: &mut HashMap<usize, usize>,
) -> (Vec<(String, String)>, usize) {
    let scroll_offset = *scroll_offsets.entry(idx).or_insert(0);

    let content = match sidebar_items.get(idx) {
        Some(SidebarItem::File(path)) => parse_env_file(path),
        Some(SidebarItem::SystemEnv) => env_vars.to_vec(),
        None => Vec::new(),
    };

    (content, scroll_offset)
}

fn main() -> std::io::Result<()> {
    let (width, height) = get_terminal_size();
    let mut renderer = Renderer::new(width, height)?;
    let mut running = true;
    let mut sidebar_scroll: usize = 0;
    let mut selected_idx: usize = 0;
    let mut focused_panel: usize = 0;
    let mut scroll_offsets: HashMap<usize, usize> = HashMap::new();

    let sidebar_items = get_sidebar_items();
    let env_vars: Vec<(String, String)> = vars_os()
        .filter_map(|(k, v)| {
            let key = k.into_string().ok()?;
            let val = v.into_string().ok()?;
            Some((key, val))
        })
        .collect();

    let (mut current_content, mut scroll_offset) =
        switch_to_item(selected_idx, &sidebar_items, &env_vars, &mut scroll_offsets);

    stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    crossterm::terminal::enable_raw_mode()?;

    // =========================================================================
    // RENDER LOOP
    // =========================================================================

    while running {
        // ---------------------------------------------------------------------
        // Terminal resize handling
        // ---------------------------------------------------------------------
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
        let _main_width = width.saturating_sub(sidebar_width);

        // ---------------------------------------------------------------------
        // Clear buffer
        // ---------------------------------------------------------------------
        buffer.clear(Rgba::BLACK);

        for y in 0..height {
            for x in 0..sidebar_width {
                buffer.draw_text(x as u32, y as u32, " ", Style::bg(Rgba::BLACK));
            }
        }

        let sidebar_visible = height.saturating_sub(2);
        let visible_rows = height.saturating_sub(4);

        // ---------------------------------------------------------------------
        // Sidebar: file list panel
        // ---------------------------------------------------------------------
        let border_color = if focused_panel == 0 { CYAN } else { GRAY };
        let sidebar_box_style = BoxStyle::single(Style::fg(border_color));
        buffer.draw_box(0, 0, sidebar_width as u32, height as u32, sidebar_box_style);

        buffer.draw_text(1, 0, "Select Source", Style::fg(GRAY));

        let sidebar_focus_indicator = if focused_panel == 0 { "» " } else { "  " };
        buffer.draw_text(1, 1, sidebar_focus_indicator, Style::fg(CYAN));

        // Write items to content panel
        for (i, item) in sidebar_items
            .iter()
            .enumerate()
            .skip(sidebar_scroll)
            .take(sidebar_visible - 2)
        {
            let line_idx = i - sidebar_scroll;
            let y = (2 + line_idx) as u32;

            let label = match item {
                SidebarItem::File(path) => path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                SidebarItem::SystemEnv => "<System Env>",
            };

            let is_selected = i == selected_idx;

            if is_selected {
                for x in 1..sidebar_width - 1 {
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

        // ---------------------------------------------------------------------
        // Content panel: variables list
        // ---------------------------------------------------------------------
        let padding = 2;
        let content_x = sidebar_width as u32 + 2;
        let content_padding_x = content_x + padding as u32;
        let _content_width = width - sidebar_width - padding;

        let content_border_color = if focused_panel == 1 { CYAN } else { GRAY };
        let content_box_style = BoxStyle::single(Style::fg(content_border_color));
        buffer.draw_box(
            content_x,
            0,
            width as u32 - content_x,
            height as u32,
            content_box_style,
        );

        buffer.draw_text(content_x + 1, 0, "Variables", Style::fg(GRAY));

        let title = match sidebar_items.get(selected_idx) {
            Some(SidebarItem::File(path)) => {
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            }
            Some(SidebarItem::SystemEnv) => "System Environment",
            None => "No source",
        };

        buffer.draw_text(content_padding_x, 1, title, Style::fg(YELLOW));

        let content_focus_indicator = if focused_panel == 1 { "» " } else { "  " };
        buffer.draw_text(content_x, 1, content_focus_indicator, Style::fg(CYAN));

        let content_vars = &current_content;

        let scroll_info = if content_vars.is_empty() {
            " (0/0)".to_string()
        } else {
            format!(" ({}/{})", scroll_offset + 1, content_vars.len())
        };
        let scroll_x = (content_padding_x as usize + title.len()) as u32;
        buffer.draw_text(scroll_x, 1, &scroll_info, Style::fg(GRAY));

        for (i, (key, val)) in content_vars
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_rows)
        {
            let line_idx = i - scroll_offset;
            let y: u32 = (3 + line_idx) as u32;

            if y as usize >= height - 1 {
                break;
            }

            let key_max_len = (width - sidebar_width - padding - 6) as usize;
            let val_max_len = (width - sidebar_width - padding - 6) as usize;

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

            buffer.draw_text(content_padding_x, y, &key_display, Style::fg(DODGER_BLUE));
            buffer.draw_text(
                content_padding_x + key_display.len() as u32 + 1,
                y,
                "=",
                Style::fg(GRAY),
            );
            buffer.draw_text(
                content_padding_x + key_display.len() as u32 + 3,
                y,
                &val_display,
                Style::fg(YELLOW),
            );
        }

        // ---------------------------------------------------------------------
        // Footer
        // ---------------------------------------------------------------------
        let help = "(arrows: move, Tab: switch, Ctrl+C: quit)";
        let footer_y = (height - 1) as u32;
        buffer.draw_text(1, footer_y, &help, Style::fg(GRAY));

        // ---------------------------------------------------------------------
        // Present frame
        // ---------------------------------------------------------------------
        renderer.present()?;

        // ---------------------------------------------------------------------
        // Input handling
        // ---------------------------------------------------------------------
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
                            scroll_offsets.insert(selected_idx, scroll_offset);
                            focused_panel = if focused_panel == 0 { 1 } else { 0 };
                            let (content, offset) = switch_to_item(
                                selected_idx,
                                &sidebar_items,
                                &env_vars,
                                &mut scroll_offsets,
                            );
                            current_content = content;
                            scroll_offset = offset;
                        }
                        KeyCode::Up => {
                            if focused_panel == 0 {
                                if sidebar_scroll > 0 && selected_idx == sidebar_scroll {
                                    sidebar_scroll -= 1;
                                }
                                if selected_idx > 0 {
                                    scroll_offsets.insert(selected_idx, scroll_offset);
                                    selected_idx -= 1;
                                    let (content, offset) = switch_to_item(
                                        selected_idx,
                                        &sidebar_items,
                                        &env_vars,
                                        &mut scroll_offsets,
                                    );
                                    current_content = content;
                                    scroll_offset = offset;
                                }
                            } else if scroll_offset >= 1 {
                                scroll_offset -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if focused_panel == 0 {
                                if selected_idx + 1 < sidebar_items.len() {
                                    scroll_offsets.insert(selected_idx, scroll_offset);
                                    selected_idx += 1;
                                    if selected_idx >= sidebar_scroll + sidebar_visible {
                                        sidebar_scroll =
                                            selected_idx.saturating_sub(sidebar_visible - 1);
                                    }
                                    let (content, offset) = switch_to_item(
                                        selected_idx,
                                        &sidebar_items,
                                        &env_vars,
                                        &mut scroll_offsets,
                                    );
                                    current_content = content;
                                    scroll_offset = offset;
                                }
                            } else if scroll_offset + visible_rows < content_vars.len() {
                                scroll_offset += 1;
                            }
                        }
                        KeyCode::PageUp => {
                            if focused_panel == 0 {
                                scroll_offsets.insert(selected_idx, scroll_offset);
                                sidebar_scroll = sidebar_scroll.saturating_sub(sidebar_visible);
                                selected_idx = sidebar_scroll;
                                let (content, offset) = switch_to_item(
                                    selected_idx,
                                    &sidebar_items,
                                    &env_vars,
                                    &mut scroll_offsets,
                                );
                                current_content = content;
                                scroll_offset = offset;
                            } else {
                                scroll_offset = scroll_offset.saturating_sub(visible_rows);
                            }
                        }
                        KeyCode::PageDown => {
                            if focused_panel == 0 {
                                scroll_offsets.insert(selected_idx, scroll_offset);
                                sidebar_scroll = (sidebar_scroll + sidebar_visible)
                                    .min(sidebar_items.len().saturating_sub(sidebar_visible));
                                selected_idx = sidebar_scroll;
                                let (content, offset) = switch_to_item(
                                    selected_idx,
                                    &sidebar_items,
                                    &env_vars,
                                    &mut scroll_offsets,
                                );
                                current_content = content;
                                scroll_offset = offset;
                            } else {
                                let max_scroll = content_vars.len().saturating_sub(visible_rows);
                                scroll_offset = (scroll_offset + visible_rows).min(max_scroll);
                            }
                        }
                        KeyCode::Left => {
                            if focused_panel == 1 {
                                focused_panel = 0;
                                let (content, offset) = switch_to_item(
                                    selected_idx,
                                    &sidebar_items,
                                    &env_vars,
                                    &mut scroll_offsets,
                                );
                                current_content = content;
                                scroll_offset = offset;
                            }
                        }
                        KeyCode::Right => {
                            if focused_panel == 0 && selected_idx + 1 < sidebar_items.len() {
                                scroll_offsets.insert(selected_idx, scroll_offset);
                                selected_idx += 1;
                                if selected_idx >= sidebar_scroll + sidebar_visible {
                                    sidebar_scroll =
                                        selected_idx.saturating_sub(sidebar_visible - 1);
                                }
                                let (content, offset) = switch_to_item(
                                    selected_idx,
                                    &sidebar_items,
                                    &env_vars,
                                    &mut scroll_offsets,
                                );
                                current_content = content;
                                scroll_offset = offset;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // ---------------------------------------------------------------------
        // End render loop
        // ---------------------------------------------------------------------
    }

    crossterm::terminal::disable_raw_mode()?;
    stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}

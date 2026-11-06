use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::anim::{self, diff_segments, EffectKind, PULSE_PERIOD};
use crate::app::{App, InputMode, Panel};
use crate::operations::OP_TYPES;

// Color palette
const ACCENT: Color = Color::Rgb(114, 137, 218); // soft blue-purple
const ACCENT_DIM: Color = Color::Rgb(78, 93, 148);
const GREEN: Color = Color::Rgb(120, 200, 120);
const YELLOW: Color = Color::Rgb(230, 200, 100);
const RED: Color = Color::Rgb(230, 100, 100);
const TEXT: Color = Color::Rgb(220, 220, 230);
const TEXT_DIM: Color = Color::Rgb(100, 100, 120);
const BG_BAR: Color = Color::Rgb(30, 30, 45);
const BG_HIGHLIGHT: Color = Color::Rgb(45, 45, 65);
const BORDER_INACTIVE: Color = Color::Rgb(55, 55, 75);
const GREEN_DIM: Color = Color::Rgb(90, 140, 90);
const RED_DIM: Color = Color::Rgb(150, 60, 60);
const WHITE: Color = Color::Rgb(255, 255, 255);

/// Blend two RGB colors; anything else falls back to `b` when `t >= 0.5`.
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            anim::lerp(r1 as f32, r2 as f32, t) as u8,
            anim::lerp(g1 as f32, g2 as f32, t) as u8,
            anim::lerp(b1 as f32, b2 as f32, t) as u8,
        ),
        _ => {
            if t >= 0.5 {
                b
            } else {
                a
            }
        }
    }
}

/// Keep only the first `n` characters of a line, span styles intact.
fn truncate_line(line: Line<'_>, n: usize) -> Line<'_> {
    let mut remaining = n;
    let mut spans = Vec::new();
    for span in line.spans {
        if remaining == 0 {
            break;
        }
        let len = span.content.chars().count();
        if len <= remaining {
            remaining -= len;
            spans.push(span);
        } else {
            let cut: String = span.content.chars().take(remaining).collect();
            spans.push(Span::styled(cut, span.style));
            remaining = 0;
        }
    }
    Line::from(spans)
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let [top_bar, main_area, preview_area, bottom_bar] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Percentage(35),
            Constraint::Length(1),
        ])
        .areas(area);

    let [files_area, ops_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .areas(main_area);

    // Startup draw-in: the bar types itself out and the panels land one by one.
    let intro = app.intro_progress();
    render_top_bar(frame, app, top_bar, intro);
    if intro >= 0.15 {
        render_file_panel(frame, app, files_area);
    }
    if intro >= 0.45 {
        render_operation_stack(frame, app, ops_area);
    }
    if intro >= 0.7 {
        render_preview(frame, app, preview_area);
    }
    if intro >= 0.9 {
        render_bottom_bar(frame, app, bottom_bar);
    }

    match app.input_mode {
        InputMode::SelectingOperationType => render_type_picker(frame, app, area),
        InputMode::EditingOperationParams => render_param_editor(frame, app, area),
        InputMode::ConfirmExecute => render_confirm(frame, app, area),
        InputMode::HelpOverlay => render_help(frame, area),
        InputMode::Normal | InputMode::Filtering | InputMode::Done => {}
    }
}

fn render_top_bar(frame: &mut Frame, app: &App, area: Rect, intro: f32) {
    let dir = app
        .files
        .first()
        .map(|f| f.parent_dir.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    let showing = if app.filtered_indices.len() != app.files.len() {
        format!(
            "  showing {}/{}",
            app.filtered_indices.len(),
            app.files.len()
        )
    } else {
        format!("  {} files", app.files.len())
    };

    let text = Line::from(vec![
        Span::styled(
            " renx ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2502} ", Style::default().fg(BORDER_INACTIVE)),
        Span::styled(dir.to_string(), Style::default().fg(TEXT)),
        Span::styled(showing, Style::default().fg(TEXT_DIM)),
        Span::styled(
            format!("  \u{2714} {}", app.selected_count()),
            Style::default().fg(GREEN),
        ),
    ]);
    let text = if intro < 1.0 {
        let total: usize = text.spans.iter().map(|s| s.content.chars().count()).sum();
        truncate_line(
            text,
            (total as f32 * intro / 0.9).min(total as f32) as usize,
        )
    } else {
        text
    };
    let bar = Paragraph::new(text).style(Style::default().bg(BG_BAR));
    frame.render_widget(bar, area);
}

fn render_file_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focused_panel == Panel::Files;
    let is_filtering = app.input_mode == InputMode::Filtering;
    let has_filter = !app.filter_text.is_empty() || is_filtering;

    // Split area: optional filter bar at top, file list below
    let (filter_area, list_area) = if has_filter {
        let [f, l] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .areas(area);
        (Some(f), l)
    } else {
        (None, area)
    };

    // Render filter bar if active
    if let Some(filter_rect) = filter_area {
        let cursor = if is_filtering { "\u{258e}" } else { "" };
        let line = Line::from(vec![
            Span::styled(
                " / ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.filter_text, Style::default().fg(TEXT)),
            Span::styled(cursor, Style::default().fg(ACCENT)),
            Span::styled(
                if is_filtering {
                    "  Esc: clear  Enter: confirm"
                } else {
                    "  / to edit  Esc: clear"
                },
                Style::default().fg(TEXT_DIM),
            ),
        ]);
        let bar = Paragraph::new(line).style(Style::default().bg(BG_HIGHLIGHT));
        frame.render_widget(bar, filter_rect);
    }

    // File list (uses filtered indices)
    let (border_color, title_style) = if focused || is_filtering {
        (
            ACCENT,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        (BORDER_INACTIVE, Style::default().fg(TEXT_DIM))
    };
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" \u{f016} ", title_style),
            Span::styled("Files ", title_style),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // Record visible height and ensure cursor is visible
    app.file_list_height = block.inner(list_area).height as usize;
    app.ensure_file_cursor_visible();

    let visible_end = (app.file_scroll + app.file_list_height).min(app.filtered_indices.len());
    let visible_slice = &app.filtered_indices[app.file_scroll..visible_end];

    let items: Vec<ListItem> = visible_slice
        .iter()
        .enumerate()
        .map(|(i, &actual_idx)| {
            let display_idx = app.file_scroll + i;
            let file = &app.files[actual_idx];
            let selected = app.selected[actual_idx];
            let is_cursor = display_idx == app.file_cursor && (focused || is_filtering);

            let base_color = if selected { GREEN } else { TEXT_DIM };
            let (check, check_color) = match app.effect_progress(&EffectKind::FilePop(actual_idx)) {
                // A toggled marker pops bright and settles into place
                Some(p) => (
                    if selected { "\u{25c9} " } else { "\u{25cc} " },
                    lerp_color(WHITE, base_color, p),
                ),
                None => (if selected { "\u{25cf} " } else { "\u{25cb} " }, base_color),
            };
            let cursor_marker = if is_cursor { "\u{25b8} " } else { "  " };

            let style = if is_cursor {
                Style::default().fg(TEXT).bg(BG_HIGHLIGHT)
            } else if selected {
                Style::default().fg(TEXT)
            } else {
                Style::default().fg(TEXT_DIM)
            };

            ListItem::new(Line::from(vec![
                Span::styled(cursor_marker, style),
                Span::styled(
                    check,
                    Style::default().fg(check_color).bg(if is_cursor {
                        BG_HIGHLIGHT
                    } else {
                        Color::Reset
                    }),
                ),
                Span::styled(&file.file_name, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, list_area);
}

fn render_operation_stack(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == Panel::Operations;
    let (border_color, title_style) = if focused {
        (
            ACCENT,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        (BORDER_INACTIVE, Style::default().fg(TEXT_DIM))
    };
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" \u{2630} ", title_style),
            Span::styled("Operations ", title_style),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.operations.is_empty() {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(TEXT_DIM)),
            Span::styled(
                "a",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to add an operation", Style::default().fg(TEXT_DIM)),
        ]))
        .block(block);
        frame.render_widget(hint, area);
        return;
    }

    let items: Vec<ListItem> = app
        .operations
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let is_cursor = i == app.op_cursor && focused;
            let num_style = if is_cursor {
                Style::default()
                    .fg(ACCENT)
                    .bg(BG_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ACCENT_DIM)
            };
            let base = if is_cursor {
                Style::default().fg(TEXT).bg(BG_HIGHLIGHT)
            } else {
                Style::default().fg(YELLOW)
            };
            // A just-added or edited operation flashes white and fades back
            let text_style = match app.effect_progress(&EffectKind::OpFlash(i)) {
                Some(p) => base
                    .fg(lerp_color(WHITE, base.fg.unwrap_or(YELLOW), p))
                    .add_modifier(Modifier::BOLD),
                None => base,
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {}. ", i + 1), num_style),
                Span::styled(op.description(), text_style),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.done.is_some() {
        render_done(frame, app, area);
        return;
    }
    let focused = app.focused_panel == Panel::Preview;
    let (border_color, title_style) = if focused {
        (
            ACCENT,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        (BORDER_INACTIVE, Style::default().fg(TEXT_DIM))
    };
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" \u{25b6} ", title_style),
            Span::styled("Preview ", title_style),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let conflicts = app.find_conflicts();

    let inner = block.inner(area);
    app.preview_height = inner.height.saturating_sub(1) as usize; // minus header line
    frame.render_widget(block, area);

    if app.preview.is_empty() {
        let msg = if app.operations.is_empty() {
            "Add an operation to see preview"
        } else {
            "No files selected"
        };
        let hint = Paragraph::new(Line::from(vec![Span::styled(
            format!("  {}", msg),
            Style::default().fg(TEXT_DIM),
        )]));
        frame.render_widget(hint, inner);
        return;
    }

    let max_old_len = app
        .preview
        .iter()
        .map(|(idx, _)| app.files[*idx].file_name.len())
        .max()
        .unwrap_or(0)
        .max(8);

    let mut lines: Vec<Line> = Vec::new();
    let header = Line::from(vec![
        Span::styled(
            format!("  {:<width$}", "Original", width = max_old_len),
            Style::default().fg(TEXT_DIM),
        ),
        Span::styled("  \u{2192}  ", Style::default().fg(BORDER_INACTIVE)),
        Span::styled("Renamed", Style::default().fg(TEXT_DIM)),
    ]);
    lines.push(header);

    let visible_count = inner.height.saturating_sub(1) as usize;
    let start = app.preview_scroll;
    let end = (start + visible_count).min(app.preview.len());

    for (display_idx, (file_idx, new)) in app.preview[start..end].iter().enumerate() {
        let actual_idx = start + display_idx;
        let is_conflict = conflicts.contains(&actual_idx);
        let old = &app.files[*file_idx].file_name;
        let is_noop = old == new;

        let new_style = if is_conflict {
            Style::default().fg(RED).add_modifier(Modifier::BOLD)
        } else if is_noop {
            Style::default().fg(TEXT_DIM)
        } else {
            Style::default().fg(GREEN)
        };

        // Rows that change get a slow pulse on the arrow, staggered per row
        let pulse = if app.animate {
            anim::pulse(app.tick + actual_idx as u64 * 3, PULSE_PERIOD)
        } else {
            1.0
        };
        let arrow_style = if is_conflict {
            Style::default().fg(lerp_color(RED_DIM, RED, pulse))
        } else if is_noop {
            Style::default().fg(BORDER_INACTIVE)
        } else {
            Style::default().fg(lerp_color(ACCENT_DIM, ACCENT, pulse))
        };

        let mut spans = vec![
            Span::styled(
                format!("  {:<width$}", old, width = max_old_len),
                Style::default().fg(TEXT),
            ),
            Span::styled("  \u{2192}  ", arrow_style),
        ];
        if is_conflict || is_noop {
            spans.push(Span::styled(new.clone(), new_style));
        } else {
            // Light up only the characters that actually changed
            for (segment, changed) in diff_segments(old, new) {
                let style = if changed {
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(GREEN_DIM)
                };
                spans.push(Span::styled(segment, style));
            }
        }
        lines.push(Line::from(spans));
    }

    if end < app.preview.len() {
        lines.push(Line::from(Span::styled(
            format!("  \u{22EE} {} more", app.preview.len() - end),
            Style::default().fg(TEXT_DIM),
        )));
    }

    let preview_text = Paragraph::new(lines);
    frame.render_widget(preview_text, inner);
}

/// Results screen: rows flip to their new names in a cascade, then a banner.
fn render_done(frame: &mut Frame, app: &App, area: Rect) {
    let Some(done) = app.done.as_ref() else {
        return;
    };
    let revealed = app.done_revealed();
    let complete = done.complete(app.tick, app.animate);
    let accent = if done.failed > 0 { YELLOW } else { GREEN };

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(
                " \u{2714} ",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Renamed ",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let max_old_len = done
        .rows
        .iter()
        .map(|r| r.old.len())
        .max()
        .unwrap_or(0)
        .max(8);

    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled(
            format!("  {:<width$}", "Original", width = max_old_len),
            Style::default().fg(TEXT_DIM),
        ),
        Span::styled("  \u{2192}  ", Style::default().fg(BORDER_INACTIVE)),
        Span::styled("Renamed", Style::default().fg(TEXT_DIM)),
    ])];

    // Header and banner take two lines; keep the newest revealed row in view
    let visible = inner.height.saturating_sub(2) as usize;
    let start = revealed
        .saturating_sub(visible)
        .min(done.rows.len().saturating_sub(visible));
    let end = (start + visible).min(done.rows.len());

    for (i, row) in done.rows.iter().enumerate().take(end).skip(start) {
        let old_span = Span::styled(
            format!("  {:<width$}", row.old, width = max_old_len),
            Style::default().fg(TEXT),
        );
        let line = match app.done_row_glow(i) {
            None => Line::from(vec![
                old_span,
                Span::styled("  \u{2026}  ", Style::default().fg(BORDER_INACTIVE)),
            ]),
            Some(glow) => {
                let (color, mark) = if row.ok {
                    (lerp_color(GREEN, WHITE, glow), "  \u{2714}")
                } else {
                    (RED, "  \u{2718} failed")
                };
                let mut new_style = Style::default().fg(color);
                if glow > 0.0 {
                    new_style = new_style.add_modifier(Modifier::BOLD);
                }
                Line::from(vec![
                    old_span,
                    Span::styled(
                        "  \u{2192}  ",
                        Style::default().fg(lerp_color(ACCENT_DIM, WHITE, glow)),
                    ),
                    Span::styled(row.new.clone(), new_style),
                    Span::styled(mark, Style::default().fg(color)),
                ])
            }
        };
        lines.push(line);
    }

    // Banner on the last line of the panel
    while lines.len() + 1 < inner.height as usize {
        lines.push(Line::from(""));
    }
    let banner = if complete {
        let summary = if done.failed == 0 {
            format!("  \u{2714} Renamed {} files", done.success)
        } else {
            format!("  Renamed {}, {} failed", done.success, done.failed)
        };
        Line::from(vec![
            Span::styled(
                summary,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   press any key to exit", Style::default().fg(TEXT_DIM)),
        ])
    } else {
        let spinner = ["\u{25d0}", "\u{25d3}", "\u{25d1}", "\u{25d2}"][(app.tick / 2 % 4) as usize];
        Line::from(vec![
            Span::styled(
                format!("  {} renaming", spinner),
                Style::default().fg(accent),
            ),
            Span::styled("   any key skips", Style::default().fg(TEXT_DIM)),
        ])
    };
    lines.push(banner);

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    // Show status message if present
    if !app.status_message.is_empty() {
        let line = Line::from(vec![Span::styled(
            format!(" {}", app.status_message),
            Style::default().fg(YELLOW),
        )]);
        let bar = Paragraph::new(line).style(Style::default().bg(BG_BAR));
        frame.render_widget(bar, area);
        return;
    }

    // Filter mode has its own bar
    if app.input_mode == InputMode::Filtering {
        let line = Line::from(vec![
            Span::styled(" Type to filter", Style::default().fg(TEXT_DIM)),
            Span::styled(" \u{2502} ", Style::default().fg(BORDER_INACTIVE)),
            Span::styled(
                "Enter",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" confirm ", Style::default().fg(TEXT_DIM)),
            Span::styled("\u{2502} ", Style::default().fg(BORDER_INACTIVE)),
            Span::styled(
                "Esc",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" clear", Style::default().fg(TEXT_DIM)),
        ]);
        let bar = Paragraph::new(line).style(Style::default().bg(BG_BAR));
        frame.render_widget(bar, area);
        return;
    }

    let hints = match app.focused_panel {
        Panel::Files => vec![
            ("Tab", "switch"),
            ("/", "filter"),
            ("Space/Enter", "toggle"),
            ("v", "all"),
            ("x", "rename"),
            ("q", "quit"),
        ],
        Panel::Operations => vec![
            ("Tab", "switch"),
            ("a", "add"),
            ("e/Enter", "edit"),
            ("d", "delete"),
            ("J/K", "reorder"),
            ("x", "rename"),
            ("q", "quit"),
        ],
        Panel::Preview => vec![
            ("Tab", "switch"),
            ("j/k", "scroll"),
            ("x", "rename"),
            ("q", "quit"),
        ],
    };

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " \u{2502} ",
                Style::default().fg(BORDER_INACTIVE),
            ));
        }
        spans.push(Span::styled(
            *key,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(TEXT_DIM),
        ));
    }

    // Right-aligned help hint
    let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
    let help_text = "? help";
    let padding = (area.width as usize)
        .saturating_sub(left_len)
        .saturating_sub(help_text.len() + 1);
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
    }
    spans.push(Span::styled(
        "?",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(" help", Style::default().fg(TEXT_DIM)));

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(BG_BAR));
    frame.render_widget(bar, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn render_confirm(frame: &mut Frame, app: &App, area: Rect) {
    let rename_count = app
        .preview
        .iter()
        .filter(|(idx, new)| app.files[*idx].file_name != *new)
        .count();

    let popup = centered_rect(44, 7, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " Confirm Rename ",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(YELLOW));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            format!(
                "  Rename {} file{}?",
                rename_count,
                if rename_count == 1 { "" } else { "s" }
            ),
            Style::default().fg(TEXT),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  y",
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" yes ", Style::default().fg(TEXT_DIM)),
            Span::styled("\u{2502} ", Style::default().fg(BORDER_INACTIVE)),
            Span::styled("n", Style::default().fg(RED).add_modifier(Modifier::BOLD)),
            Span::styled(" no", Style::default().fg(TEXT_DIM)),
        ]),
    ];

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

fn render_type_picker(frame: &mut Frame, app: &App, area: Rect) {
    let height = (OP_TYPES.len() as u16 + 2).min(area.height);
    let popup = centered_rect(40, height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " Add Operation ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));

    let items: Vec<ListItem> = OP_TYPES
        .iter()
        .enumerate()
        .map(|(i, spec)| {
            let name = spec.name;
            let style = if i == app.popup_type_cursor {
                Style::default()
                    .fg(ACCENT)
                    .bg(BG_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            let marker = if i == app.popup_type_cursor {
                " \u{25b8} "
            } else {
                "   "
            };
            ListItem::new(format!("{}{}", marker, name)).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, popup);
}

fn render_param_editor(frame: &mut Frame, app: &App, area: Rect) {
    let field_count = app.popup_fields.len() as u16;
    let height = 4 + field_count * 2;
    let popup = centered_rect(48, height, area);
    frame.render_widget(Clear, popup);

    let title = if app.editing_op_index.is_some() {
        " Edit Operation "
    } else {
        " Operation Parameters "
    };
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let labels = get_field_labels(app);

    let mut lines: Vec<Line> = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        let is_focused = i == app.popup_field_cursor;
        let label_style = Style::default().fg(TEXT_DIM);

        lines.push(Line::from(Span::styled(
            format!("  {}", label),
            label_style,
        )));

        // Check if this field is a selector
        let is_selector = app
            .popup_field_options
            .get(i)
            .map(|o| o.is_some())
            .unwrap_or(false);

        if is_selector {
            let options = app.popup_field_options[i].as_ref().unwrap();
            let sel = app.popup_field_selections.get(i).copied().unwrap_or(0);
            let mut spans = vec![Span::raw("  ")];
            if is_focused {
                spans.push(Span::styled("\u{25c0} ", Style::default().fg(ACCENT)));
            } else {
                spans.push(Span::raw("  "));
            }
            for (j, opt) in options.iter().enumerate() {
                if j > 0 {
                    spans.push(Span::styled("  ", Style::default()));
                }
                if j == sel {
                    spans.push(Span::styled(
                        opt.clone(),
                        Style::default()
                            .fg(ACCENT)
                            .bg(BG_HIGHLIGHT)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::styled(opt.clone(), Style::default().fg(TEXT_DIM)));
                }
            }
            if is_focused {
                spans.push(Span::styled(" \u{25b6}", Style::default().fg(ACCENT)));
            }
            lines.push(Line::from(spans));
        } else {
            // Text input field — render as a visible input box with cursor
            let value = app.popup_fields.get(i).map(|s| s.as_str()).unwrap_or("");
            let text_cursor = app
                .popup_text_cursors
                .get(i)
                .copied()
                .unwrap_or(value.len());
            let field_width: usize = 38;

            let bg = if is_focused {
                BG_HIGHLIGHT
            } else {
                Color::Rgb(35, 35, 50)
            };
            let fg = if is_focused { TEXT } else { TEXT_DIM };
            let border_color = if is_focused { ACCENT } else { BORDER_INACTIVE };

            let mut spans = vec![
                Span::raw("  "),
                Span::styled("\u{2502}", Style::default().fg(border_color)),
                Span::styled(" ", Style::default().bg(bg)),
            ];

            if is_focused {
                let byte_cursor = crate::app::char_to_byte_index(
                    value,
                    text_cursor.min(crate::app::char_len(value)),
                );
                let before = &value[..byte_cursor];
                let after = &value[byte_cursor..];
                spans.push(Span::styled(before, Style::default().fg(fg).bg(bg)));
                spans.push(Span::styled("\u{258e}", Style::default().fg(ACCENT).bg(bg)));
                let pad_len = field_width.saturating_sub(before.len() + after.len() + 1);
                spans.push(Span::styled(after, Style::default().fg(fg).bg(bg)));
                spans.push(Span::styled(" ".repeat(pad_len), Style::default().bg(bg)));
            } else {
                let pad_len = field_width.saturating_sub(value.len());
                spans.push(Span::styled(value, Style::default().fg(fg).bg(bg)));
                spans.push(Span::styled(" ".repeat(pad_len), Style::default().bg(bg)));
            }

            spans.push(Span::styled("\u{2502}", Style::default().fg(border_color)));
            lines.push(Line::from(spans));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  Enter",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" confirm ", Style::default().fg(TEXT_DIM)),
        Span::styled("\u{2502} ", Style::default().fg(BORDER_INACTIVE)),
        Span::styled(
            "Esc",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" cancel ", Style::default().fg(TEXT_DIM)),
        Span::styled("\u{2502} ", Style::default().fg(BORDER_INACTIVE)),
        Span::styled(
            "Tab",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" next", Style::default().fg(TEXT_DIM)),
    ]));

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

fn get_field_labels(app: &App) -> Vec<&'static str> {
    OP_TYPES
        .get(app.popup_type_cursor)
        .map(|spec| spec.fields.iter().map(|f| f.label).collect())
        .unwrap_or_default()
}

fn render_help(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(62, 34, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " Help ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    let sections = vec![
        (
            "  Navigation",
            vec![
                ("Tab / Shift+Tab", "Cycle panels forward / back"),
                ("j/k  \u{2191}/\u{2193}", "Move cursor up/down"),
                ("Ctrl+d / Ctrl+u", "Half-page down / up"),
                ("PgDn / PgUp", "Full page down / up"),
                ("g / G", "Jump to top / bottom"),
            ],
        ),
        (
            "  File List",
            vec![
                ("Space / Enter", "Toggle file selection"),
                ("v", "Select / deselect all visible"),
                ("/", "Filter files (wildcards: * ?)"),
                ("Esc", "Clear filter"),
            ],
        ),
        (
            "  Operations",
            vec![
                ("a", "Add new operation"),
                ("e / Enter", "Edit selected operation"),
                ("d", "Delete selected operation"),
                ("J/K  Shift+\u{2191}/\u{2193}", "Reorder operation up/down"),
            ],
        ),
        (
            "  Execute",
            vec![
                ("x", "Rename files (with confirmation)"),
                ("X", "Force rename (skip confirmation)"),
            ],
        ),
        ("  General", vec![("?", "Toggle this help"), ("q", "Quit")]),
    ];

    for (heading, items) in sections {
        lines.push(Line::from(Span::styled(
            heading,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in items {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {:<20}", key),
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc, Style::default().fg(TEXT_DIM)),
            ]));
        }
        lines.push(Line::from(""));
    }

    let content = Paragraph::new(lines).block(block);
    frame.render_widget(content, popup);
}

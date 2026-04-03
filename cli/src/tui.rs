use crate::control::TelemetryEvent;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use std::io;
use tokio_stream::StreamExt;

struct AppState {
    events: Vec<TelemetryEvent>,
    list_state: ListState,
}

impl AppState {
    fn new() -> AppState {
        AppState {
            events: vec![],
            list_state: ListState::default(),
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.events.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.events.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

use crossterm::event::EventStream;

pub async fn run_tui(mut stream: tonic::Streaming<TelemetryEvent>, env_id: &str) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    let mut reader = EventStream::new();

    loop {
        terminal.draw(|f| draw_ui(f, &mut app, env_id))?;

        tokio::select! {
            // Read from gRPC Stream
            message = stream.next() => {
                match message {
                    Some(Ok(event)) => {
                        app.events.push(event);
                        // Auto-scroll logic if nothing is selected or we're following the tail
                        if app.list_state.selected().is_none() {
                            let len = app.events.len();
                            app.list_state.select(Some(len.saturating_sub(1)));
                        } else if let Some(selected) = app.list_state.selected() {
                            if selected == app.events.len().saturating_sub(2) {
                                let len = app.events.len();
                                app.list_state.select(Some(len.saturating_sub(1)));
                            }
                        }
                    }
                    Some(Err(e)) => {
                        app.events.push(TelemetryEvent {
                            event_type: "Fatal".into(),
                            content: format!("gRPC Stream Error: {}", e),
                            agent_name: "Control Plane".into()
                        });
                    }
                    None => {
                        // Stream closed
                        app.events.push(TelemetryEvent {
                            event_type: "System".into(),
                            content: "Daemon closed the telemetry connection.".into(),
                            agent_name: "Control Plane".into()
                        });
                    }
                }
            }
            // Read Terminal Keyboard Events
            maybe_event = reader.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_ui(f: &mut ratatui::Frame, app: &mut AppState, env_id: &str) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.area());

    // Left Pane (The Timeline)
    let items: Vec<ListItem> = app
        .events
        .iter()
        .map(|e| {
            let color = match e.event_type.as_str() {
                "Reasoning" => Color::Cyan,
                "Tool Request" => Color::Yellow,
                "Tool Result" => Color::Green,
                "Error" | "Fatal" => Color::Red,
                _ => Color::DarkGray,
            };

            let header = Line::from(vec![
                Span::styled(
                    format!("[{}] ", e.agent_name),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(e.event_type.clone(), Style::default().fg(color)),
            ]);
            // Take the first line of content as preview
            let preview = e
                .content
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(40)
                .collect::<String>();
            // Add ellipses if > 40 chars
            let preview_span = Span::styled(
                format!("  {}...", preview),
                Style::default().fg(Color::DarkGray),
            );

            ListItem::new(vec![header, Line::from(preview_span)])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Timeline: {}", env_id))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[0], &mut app.list_state);

    // Right Pane (Deep Content Inspect)
    let content = if let Some(i) = app.list_state.selected() {
        if let Some(event) = app.events.get(i) {
            event.content.clone()
        } else {
            String::new()
        }
    } else {
        "Awaiting environment telemetry...".to_string()
    };

    let p = Paragraph::new(tui_markdown::from_str(&content))
        .block(Block::default().title("Inspector").borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    f.render_widget(p, chunks[1]);
}

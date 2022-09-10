use datatracker_rust::stats::GetStatsRequest;
use datatracker_rust::stats::{stats_client::StatsClient, GetStatsResponse};
use std::{sync::Arc, time::Duration};
use tokio::signal;
use tokio::sync::Mutex;

fn get_stats_response_to_str(gsr: &GetStatsResponse) -> Vec<&str> {
    vec![gsr.id.as_str(), gsr.name.as_str(), gsr.input.as_str()]
}

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame, Terminal,
};

struct App {
    state: TableState,
    items: Vec<GetStatsResponse>,
    s: Arc<Mutex<Vec<GetStatsResponse>>>,
    picked: Option<usize>,
}

impl App {
    async fn new(state: Arc<Mutex<Vec<GetStatsResponse>>>) -> App {
        App {
            state: TableState::default(),
            s: state.clone(),
            items: state.lock().await.to_owned(),
            picked: None,
        }
    }
    pub fn next(&mut self) {
        if self.picked.is_some() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.picked.is_some() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub async fn reload(&mut self) {
        self.items = self.s.lock().await.to_owned();
    }

    pub fn pick(&mut self) {
        if self.picked.is_some() {
            return;
        }
        if let Some(i) = self.state.selected() {
            self.picked = Some(i)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut client = StatsClient::connect("http://[::1]:10000").await.unwrap();
    let mut timer = tokio::time::interval(Duration::new(5, 0));
    let state: Arc<Mutex<Vec<GetStatsResponse>>> = Arc::default();

    let state_cloned = state.clone();
    tokio::task::spawn(async move {
        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    return;
                }
                _ = timer.tick() => {
                    let mut stream = client.get_stats(GetStatsRequest::default()).await.unwrap().into_inner();
                    let mut v = vec![];
                    while let Some(feature) = stream.message().await.unwrap() {
                        v.push(feature);
                    }
                    let mut s = state_cloned.lock().await;
                    *s = v;
                }
            }
        }
    });

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new(state.clone()).await;
    run_app(&mut terminal, app).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) {
    loop {
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        app.reload().await;

        if let Event::Key(key) = event::read().unwrap() {
            match key.code {
                KeyCode::Char('q') => {
                    if app.picked.is_none() {
                        return;
                    }
                    app.picked = None;
                }
                KeyCode::Enter => app.pick(),
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                _ => {}
            }
        }
    }
}

fn create_headers_cells<'a>(headers: &[&'a str]) -> Row<'a> {
    let normal_style = Style::default().bg(Color::Blue);
    let header_cells = headers
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);
    header
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());
    let selected_style = Style::default().add_modifier(Modifier::REVERSED);

    let t: Table = match app.picked {
        Some(i) => {
            let rows = vec![Row::new(vec![Cell::from(app.items[i].eval_forest.clone())])
                .height(10_u16)
                .bottom_margin(1)];
            Table::new(rows)
                .header(create_headers_cells(&["definition"]))
                .block(Block::default().borders(Borders::ALL).title("Table"))
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[Constraint::Percentage(100)])
        }
        None => {
            let rows = app.items.iter().map(|item| {
                let items = get_stats_response_to_str(item);
                let height = items
                    .iter()
                    .map(|content| content.chars().filter(|c| *c == '\n').count())
                    .max()
                    .unwrap_or(0)
                    + 1;
                let cells = items.iter().map(|c| Cell::from(*c));
                Row::new(cells).height(height as u16).bottom_margin(1)
            });
            Table::new(rows)
                .header(create_headers_cells(&["uuid", "name", "url"]))
                .block(Block::default().borders(Borders::ALL).title("Table"))
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[
                    Constraint::Percentage(20),
                    Constraint::Percentage(40),
                    Constraint::Percentage(40),
                ])
        }
    };
    f.render_stateful_widget(t, rects[0], &mut app.state);
}

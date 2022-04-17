use datatracker_rust::stats::GetStatsRequest;
use datatracker_rust::stats::{stats_client::StatsClient, GetStatsResponse};
use std::{sync::Arc, time::Duration};
use tokio::signal;
use tokio::sync::Mutex;

fn get_stats_response_to_str<'a>(gsr: &'a GetStatsResponse) -> Vec<&'a str> {
    let mut v = vec![];
    v.push(gsr.id.as_str());
    v.push(gsr.name.as_str());
    v.push(gsr.url.as_str());
    v
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
}

impl App {
    async fn new(state: Arc<Mutex<Vec<GetStatsResponse>>>) -> App {
        App {
            state: TableState::default(),
            s: state.clone(),
            items: state.lock().await.to_owned(),
        }
    }
    pub fn next(&mut self) {
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
                KeyCode::Char('q') => return,
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default().bg(Color::Blue);
    let header_cells = ["Header1", "Header2", "Header3", "Header4"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);
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
    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Table"))
        .highlight_style(selected_style)
        .highlight_symbol(">> ")
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Length(30),
            Constraint::Min(10),
        ]);
    f.render_stateful_widget(t, rects[0], &mut app.state);
}

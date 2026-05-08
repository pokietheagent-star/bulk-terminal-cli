mod app;
mod market;
mod news;
mod ui;

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::{App, AppEvent, SortMode, Timeframe};

fn main() -> Result<()> {
    let (event_tx, event_rx) = mpsc::channel::<AppEvent>();

    market::spawn_market_thread(event_tx.clone());
    spawn_news_thread(event_tx.clone());

    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, event_rx);
    restore_terminal(&mut terminal)?;
    result
}

fn spawn_news_thread(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        let mut poller = news::NewsPoller::default();
        loop {
            let snapshot = poller.poll();
            if tx.send(AppEvent::News(snapshot)).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(45));
        }
    });
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    event_rx: Receiver<AppEvent>,
) -> Result<()> {
    let mut app = App::new();
    let mut last_draw = Instant::now();

    loop {
        while let Ok(event) = event_rx.try_recv() {
            app.apply_event(event);
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up | KeyCode::Char('k') => app.select_prev_symbol(),
                        KeyCode::Down | KeyCode::Char('j') => app.select_next_symbol(),
                        KeyCode::Left | KeyCode::Char('h') => app.prev_news(),
                        KeyCode::Right | KeyCode::Char('l') => app.next_news(),
                        KeyCode::Char('a') => app.toggle_active_symbol_filter(),
                        KeyCode::Char('s') => app.toggle_severity_filter(),
                        KeyCode::Char('g') => app.cycle_sort_mode(SortMode::Symbol),
                        KeyCode::Char('m') => app.cycle_sort_mode(SortMode::Move),
                        KeyCode::Char('f') => app.cycle_sort_mode(SortMode::Funding),
                        KeyCode::Char('o') => app.cycle_sort_mode(SortMode::OpenInterest),
                        KeyCode::Char('p') => app.cycle_sort_mode(SortMode::Spread),
                        KeyCode::Char('1') => app.set_timeframe(Timeframe::M1),
                        KeyCode::Char('5') => app.set_timeframe(Timeframe::M5),
                        KeyCode::Char('t') => app.set_timeframe(Timeframe::M15),
                        KeyCode::Char('y') => app.set_timeframe(Timeframe::H1),
                        _ => {}
                    }
                }
            }
        }

        if last_draw.elapsed() >= Duration::from_millis(120) {
            terminal.draw(|frame| ui::draw(frame, &app))?;
            last_draw = Instant::now();
        }
    }
}

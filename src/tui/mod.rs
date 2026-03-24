mod app;
mod event;
mod render;
mod tree;
mod virtual_scroll;

use std::{
    io,
    path::PathBuf,
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

use crossterm::{
    event as ct_event, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{Terminal, backend::CrosstermBackend};

pub use app::App;

const WATCH_POLL_INTERVAL_MS: u64 = 500;

pub fn run_tui(file_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let app = App::from_file(file_path)?;
    run_loop(app)?;
    Ok(())
}

fn run_loop(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, ct_event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let file_path = app.file_path.clone();
    let (watcher_tx, watch_rx) = channel::<notify::Result<notify::Event>>();

    let mut watcher = RecommendedWatcher::new(watcher_tx, Config::default())?;
    watcher.watch(&file_path, RecursiveMode::NonRecursive)?;
    drop(watcher);

    let result = event_loop(&mut terminal, &mut app, watch_rx);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        ct_event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

#[allow(clippy::needless_pass_by_value)]
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    watch_rx: Receiver<notify::Result<notify::Event>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let poll_interval = Duration::from_millis(WATCH_POLL_INTERVAL_MS);

    loop {
        terminal.draw(|frame| render::render(frame, app))?;

        let timeout = if app.file_changed.is_some() {
            Duration::ZERO
        } else {
            poll_interval
        };

        if ct_event::poll(timeout)? {
            let evt = ct_event::read()?;
            event::handle_event(app, &evt);
        }

        match watch_rx.try_recv() {
            Ok(Ok(event))
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() =>
            {
                if app.check_file_changed() {
                    let locale = crate::i18n::get_locale();
                    let msg = crate::i18n::t_to("tui.status.file_changed", &locale);
                    app.set_status(&msg, app::StatusLevel::Warn);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {e}");
            }
            _ => {}
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

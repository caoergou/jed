mod app;
mod event;
mod render;
mod tree;

use std::{io, path::PathBuf, time::Duration};

use crossterm::{
    event as ct_event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub use app::App;

/// 启动 TUI 会话。
///
/// 处理终端的初始化和清理，确保任何退出路径都能恢复终端状态。
pub fn run_tui(file_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let app = App::from_file(file_path)?;
    run_loop(app)?;
    Ok(())
}

fn run_loop(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    // 初始化终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    // 无论是否出错，都要清理终端
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| render::render(frame, app))?;

        if ct_event::poll(Duration::from_millis(50))? {
            let evt = ct_event::read()?;
            event::handle_event(app, evt);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

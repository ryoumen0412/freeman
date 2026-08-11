//! Freeman TUI - Actor-based API testing tool
//! GPLv3
//! Architecture:
//! - UI Layer (Ratatui) - synchronous terminal rendering
//! - App Layer - central state machine processing events
//! - Network Layer (Tokio) - async HTTP execution

mod app;
mod constants;
mod curl;
mod discovery;
mod messages;
mod models;
mod network;
mod storage;
mod tui;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use app::AppActor;
use messages::ui_events::key_to_ui_event;
use messages::{NetworkCommand, NetworkResponse, RenderState, UiEvent};
use network::NetworkActor;
use terminal_colorsaurus::{color_scheme, ColorScheme, QueryOptions};
use tui::theme::{Theme, GLOBAL_THEME};

/// Terminal cleanup guard
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to file
    let file_appender = tracing_appender::rolling::never(".", "freeman.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    // Install panic hook BEFORE enabling raw mode.
    // This ensures the terminal is restored even on panic.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(panic_info);
    }));

    // Detect terminal theme (light/dark)
    let detected_theme = match color_scheme(QueryOptions::default()) {
        Ok(ColorScheme::Light) => Theme::light(),
        _ => Theme::dark(),
    };
    let _ = GLOBAL_THEME.set(detected_theme);

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channels
    let (ui_tx, ui_rx) = mpsc::unbounded_channel::<UiEvent>();
    let (net_cmd_tx, net_cmd_rx) = mpsc::unbounded_channel::<NetworkCommand>();
    let (net_resp_tx, net_resp_rx) = mpsc::unbounded_channel::<NetworkResponse>();
    let (render_tx, mut render_rx) = mpsc::unbounded_channel::<std::sync::Arc<RenderState>>();

    // Spawn network actor
    let network_actor = NetworkActor::new(net_resp_tx);
    tokio::spawn(network_actor.run(net_cmd_rx));

    // Spawn app actor
    let app_actor = AppActor::new(net_cmd_tx, render_tx);
    tokio::spawn(app_actor.run(ui_rx, net_resp_rx));

    // Run UI loop (synchronous with async polling)
    run_ui_loop(&mut terminal, ui_tx, &mut render_rx).await?;

    Ok(())
}

/// Run the synchronous UI rendering loop
async fn run_ui_loop(
    terminal: &mut Terminal<impl Backend>,
    ui_tx: mpsc::UnboundedSender<UiEvent>,
    render_rx: &mut mpsc::UnboundedReceiver<std::sync::Arc<RenderState>>,
) -> anyhow::Result<()> {
    let mut current_state = std::sync::Arc::new(RenderState::default());

    loop {
        // Draw with current state
        terminal.draw(|f| tui::draw::draw_ui(f, &current_state))?;

        // Poll for events with timeout
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let Some(event) = key_to_ui_event(
                    key,
                    current_state.active_tab,
                    current_state.http.active_panel,
                    current_state.input_mode,
                    current_state.show_help,
                    current_state.http.show_curl_import,
                    current_state.http.show_workspace_input,
                ) {
                    if matches!(event, UiEvent::Quit) {
                        let _ = ui_tx.send(event);
                        break;
                    }
                    let _ = ui_tx.send(event);
                }
            }
        }

        // Check for state updates (non-blocking)
        while let Ok(state) = render_rx.try_recv() {
            current_state = state;
        }
    }

    Ok(())
}

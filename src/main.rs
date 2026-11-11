#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adblock;
mod app;
mod assets;
mod data;
mod defaults;
mod downloads;
mod engines;
mod importer;
mod internal;
mod ipc;
mod logo;
mod store;
mod urlbar;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};

fn main() {
    let initial_url = std::env::args().skip(1).find(|arg| {
        !arg.is_empty() && !arg.starts_with("--") && !arg.eq_ignore_ascii_case("deeps.exe")
    });

    let event_loop = EventLoopBuilder::<app::AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut app: Option<app::App> = None;
    let initial_url = initial_url.clone();

    let _ = event_loop.run(move |event, target, control_flow| {
        if matches!(&event, Event::NewEvents(StartCause::Init)) {
            match app::App::new(target, proxy.clone(), initial_url.clone()) {
                Ok(instance) => app = Some(instance),
                Err(error) => {
                    eprintln!("deeps: failed to start: {error}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
        }

        if let Some(instance) = app.as_mut() {
            instance.handle_event(event, control_flow);
        }
    });
}

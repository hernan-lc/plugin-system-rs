use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "linux")]
use tao::platform::unix::EventLoopBuilderExtUnix;
#[cfg(target_os = "windows")]
use tao::platform::windows::EventLoopBuilderExtWindows;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

fn create_icon_rgba() -> tray_icon::Icon {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let cx = x as f32 - size as f32 / 2.0;
            let cy = y as f32 - size as f32 / 2.0;
            let dist = (cx * cx + cy * cy).sqrt();

            if dist < size as f32 / 2.0 - 1.0 {
                if (6..=25).contains(&x) && (6..=25).contains(&y) {
                    rgba[idx] = 0;
                    rgba[idx + 1] = 212;
                    rgba[idx + 2] = 255;
                    rgba[idx + 3] = 255;
                } else {
                    rgba[idx] = 45;
                    rgba[idx + 1] = 45;
                    rgba[idx + 2] = 45;
                    rgba[idx + 3] = 255;
                }
            } else {
                rgba[idx + 3] = 0;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

fn get_local_ip() -> String {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

pub fn spawn_tray(shutdown: Arc<AtomicBool>, pid_lock_path: PathBuf, port: u16) {
    thread::spawn(move || {
        let mut builder = EventLoopBuilder::<()>::new();
        #[cfg(target_os = "linux")]
        builder.with_any_thread(true);
        #[cfg(target_os = "windows")]
        builder.with_any_thread(true);
        let event_loop = builder.build();

        let tray_menu = Menu::new();

        let url = format!("http://{}:{port}", get_local_ip());

        let open_item = MenuItem::with_id("open", "Open in Browser", true, None);
        let status_item = MenuItem::with_id("status", format!("Server: {}", url), false, None);
        let exit_item = MenuItem::with_id("exit", "Exit", true, None);

        tray_menu.append(&status_item).unwrap();
        tray_menu.append(&PredefinedMenuItem::separator()).unwrap();
        tray_menu.append(&open_item).unwrap();
        tray_menu.append(&PredefinedMenuItem::separator()).unwrap();
        tray_menu.append(&exit_item).unwrap();

        let icon = create_icon_rgba();

        let _tray_icon: TrayIcon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("StreamDeck Core")
            .with_icon(icon)
            .with_menu_on_left_click(true)
            .build()
            .expect("Failed to create tray icon");

        println!("Tray icon created at {}", url);

        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();

        event_loop.run(move |_event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            if shutdown.load(Ordering::Relaxed) {
                *control_flow = ControlFlow::Exit;
                return;
            }

            while let Ok(event) = menu_channel.try_recv() {
                match event.id().0.as_str() {
                    "open" => {
                        let url = format!("http://localhost:{port}");
                        println!("Opening browser: {}", url);
                        let _ = open::that(&url);
                    }
                    "exit" => {
                        println!("Exit requested from tray");
                        shutdown.store(true, Ordering::Relaxed);
                        let _ = std::fs::remove_file(&pid_lock_path);
                        *control_flow = ControlFlow::Exit;
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }

            let _ = tray_channel.try_recv();
        });
    });
}

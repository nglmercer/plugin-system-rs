use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "linux")]
use tao::platform::unix::EventLoopBuilderExtUnix;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

const PORT: u16 = 3000;

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
                if x >= 6 && x <= 25 && y >= 6 && y <= 25 {
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

pub fn spawn_tray(shutdown: Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut builder = EventLoopBuilder::<()>::new();
        #[cfg(target_os = "linux")]
        builder.with_any_thread(true);
        let event_loop = builder.build();

        let tray_menu = Menu::new();

        let local_ip = get_local_ip();
        let url = format!("http://{}:{}", local_ip, PORT);

        let open_item = MenuItem::with_id("open", "Open in Browser", true, None);
        let qr_item = MenuItem::with_id("qr", "Show QR Code (mobile)", true, None);
        let copy_url_item = MenuItem::with_id("copy_url", &format!("Copy URL: {}", url), true, None);
        let status_item = MenuItem::with_id("status", &format!("Server: {}", url), false, None);
        let exit_item = MenuItem::with_id("exit", "Exit", true, None);

        tray_menu.append(&status_item).unwrap();
        tray_menu.append(&PredefinedMenuItem::separator()).unwrap();
        tray_menu.append(&open_item).unwrap();
        tray_menu.append(&qr_item).unwrap();
        tray_menu.append(&copy_url_item).unwrap();
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

            loop {
                match menu_channel.try_recv() {
                    Ok(event) => {
                        match event.id().0.as_str() {
                            "open" => {
                                let url = format!("http://localhost:{}", PORT);
                                println!("Opening browser: {}", url);
                                let _ = open::that(&url);
                            }
                            "qr" => {
                                let url = format!("http://{}:{}", get_local_ip(), PORT);
                                println!("\n=== QR CODE ===");
                                println!("Scan to connect from mobile:");
                                println!("{}", generate_qr_text(&url));
                                println!("URL: {}\n", url);
                            }
                            "copy_url" => {
                                let url = format!("http://{}:{}", get_local_ip(), PORT);
                                println!("URL copied: {}", url);
                                #[cfg(target_os = "linux")]
                                {
                                    let _ = std::process::Command::new("xclip")
                                        .args(["-selection", "clipboard"])
                                        .arg("-i")
                                        .stdin(std::process::Stdio::piped())
                                        .spawn()
                                        .and_then(|mut child| {
                                            use std::io::Write;
                                            if let Some(stdin) = child.stdin.as_mut() {
                                                stdin.write_all(url.as_bytes())?;
                                            }
                                            child.wait().map(|_| ())
                                        });
                                }
                            }
                            "exit" => {
                                println!("Exit requested from tray");
                                shutdown.store(true, Ordering::Relaxed);
                                *control_flow = ControlFlow::Exit;
                                std::process::exit(0);
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }
            }

            let _ = tray_channel.try_recv();
        });
    });
}

fn generate_qr_text(url: &str) -> String {
    let mut result = String::new();
    let modules = encode_qr(url);
    let size = modules.len();

    result.push_str("  ");
    for _ in 0..size + 4 {
        result.push('\u{2588}');
        result.push('\u{2588}');
    }
    result.push('\n');

    for _ in 0..2 {
        result.push_str("  ");
        result.push('\u{2588}');
        result.push('\u{2588}');
        for _ in 0..size {
            result.push('\u{2588}');
            result.push('\u{2588}');
        }
        result.push('\u{2588}');
        result.push('\u{2588}');
        result.push('\n');
    }

    for row in &modules {
        result.push_str("  ");
        result.push('\u{2588}');
        result.push('\u{2588}');
        for &cell in row {
            if cell {
                result.push(' ');
                result.push(' ');
            } else {
                result.push('\u{2588}');
                result.push('\u{2588}');
            }
        }
        result.push('\u{2588}');
        result.push('\u{2588}');
        result.push('\n');
    }

    for _ in 0..2 {
        result.push_str("  ");
        result.push('\u{2588}');
        result.push('\u{2588}');
        for _ in 0..size {
            result.push('\u{2588}');
            result.push('\u{2588}');
        }
        result.push('\u{2588}');
        result.push('\u{2588}');
        result.push('\n');
    }

    result.push_str("  ");
    for _ in 0..size + 4 {
        result.push('\u{2588}');
        result.push('\u{2588}');
    }
    result.push('\n');

    result
}

fn encode_qr(data: &str) -> Vec<Vec<bool>> {
    let size = 25;
    let mut modules = vec![vec![false; size]; size];

    add_finder_pattern(&mut modules, 0, 0);
    add_finder_pattern(&mut modules, size - 7, 0);
    add_finder_pattern(&mut modules, 0, size - 7);

    let bytes = data.as_bytes();
    let mut bit_idx = 0;
    let mut col = size - 1;
    let mut going_up = true;

    while col > 0 {
        if col == 6 {
            col -= 1;
            continue;
        }

        let rows: Vec<usize> = if going_up {
            (0..size).collect()
        } else {
            (0..size).rev().collect()
        };

        for &row in &rows {
            if is_reserved(row, col, size) && is_reserved(row, col - 1, size) {
                continue;
            }

            if !is_reserved(row, col, size) && bit_idx < bytes.len() * 8 {
                let byte_idx = bit_idx / 8;
                let bit_pos = 7 - (bit_idx % 8);
                modules[row][col] = (bytes[byte_idx] >> bit_pos) & 1 == 1;
                bit_idx += 1;
            }

            if col > 0 && !is_reserved(row, col - 1, size) {
                if bit_idx < bytes.len() * 8 {
                    let byte_idx = bit_idx / 8;
                    let bit_pos = 7 - (bit_idx % 8);
                    modules[row][col - 1] = (bytes[byte_idx] >> bit_pos) & 1 == 1;
                    bit_idx += 1;
                }
            }
        }

        going_up = !going_up;
        col -= 2;
    }

    modules
}

fn is_reserved(row: usize, col: usize, size: usize) -> bool {
    if row < 9 && col < 9 {
        return true;
    }
    if row < 9 && col >= size - 8 {
        return true;
    }
    if row >= size - 8 && col < 9 {
        return true;
    }
    if row == 6 || col == 6 {
        return true;
    }
    false
}

fn add_finder_pattern(modules: &mut Vec<Vec<bool>>, start_row: usize, start_col: usize) {
    for r in 0..7 {
        for c in 0..7 {
            let is_black = r == 0
                || r == 6
                || c == 0
                || c == 6
                || (r >= 2 && r <= 4 && c >= 2 && c <= 4);
            if start_row + r < modules.len() && start_col + c < modules[0].len() {
                modules[start_row + r][start_col + c] = is_black;
            }
        }
    }

    if start_row > 0 {
        for c in 0..8 {
            if start_col + c < modules[0].len() && start_row + 7 < modules.len() {
                modules[start_row + 7][start_col + c] = false;
            }
        }
    }
    if start_col > 0 {
        for r in 0..8 {
            if start_row + r < modules.len() && start_col + 7 < modules[0].len() {
                modules[start_row + r][start_col + 7] = false;
            }
        }
    }
}

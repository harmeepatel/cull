#![allow(warnings, dead_code)]
use std::path::PathBuf;

use crossbeam_channel;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::html::{FileData, HasFileData};
use dioxus::prelude::*;
use dioxus_desktop::LogicalSize;

mod config;
mod sorter;

use sorter::{SortEvent, SortMode};


fn main() {
    let window = WindowBuilder::new()
        .with_transparent(true)
        .with_min_inner_size(LogicalSize::new(600, 500))
        .with_always_on_top(false)
        .with_title("CULL");
    let cfg = Config::default().with_window(window.with_background_color((0, 0, 0, 0)));
    dioxus::LaunchBuilder::new().with_cfg(cfg).launch(App);
}

fn apply_blur() {
    #[cfg(target_os = "macos")]
    unsafe {
        use cocoa::base::{id, nil, NO};
        use cocoa::foundation::NSRect;
        use dioxus_desktop::{tao::platform::macos::WindowExtMacOS as _, window};
        use objc::*;

        let desktop = window();
        let ns_window = desktop.window.ns_window() as id;
        let content_view: id = msg_send![ns_window, contentView];
        let frame: NSRect = msg_send![content_view, frame];

        let cls = objc::runtime::Class::get("NSVisualEffectView").unwrap();
        let vev: id = msg_send![cls, alloc];
        let vev: id = msg_send![vev, initWithFrame: frame];

        let () = msg_send![vev, setMaterial: 3i64];
        let () = msg_send![vev, setBlendingMode: 0i64];
        let () = msg_send![vev, setState: 1i64];
        let () = msg_send![vev, setAutoresizingMask: 18usize];
        let () = msg_send![ns_window, setOpaque: NO];
        let () = msg_send![ns_window, setBackgroundColor: nil];
        let () = msg_send![content_view, addSubview: vev positioned: -1i64 relativeTo: nil];
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Idle,
    Sorting,
    Done,
}

#[component]
fn App() -> Element {
    use_effect(move || {
        apply_blur();
    });

    const BG: &str = "var(--toggle-bg)";
    const HIGHLIGHT: &str = "rgba(var(--highlight), var(--alpha))";

    let mut drop_bg = use_signal(|| BG.to_string());
    let mut dest = use_signal(|| config::load_dest().unwrap_or_default());
    let mut mode = use_signal(|| SortMode::Copy);
    let mut phase = use_signal(|| Phase::Idle);
    let mut progress_done = use_signal(|| 0usize);
    let mut progress_total = use_signal(|| 0usize);
    let mut result_ok = use_signal(|| 0usize);
    let mut result_errors = use_signal(|| 0usize);

    let dest_display = {
        let p = dest.read();
        if p.as_os_str().is_empty() {
            "No folder selected".to_string()
        } else {
            p.to_string_lossy().to_string()
        }
    };

    let mut on_files = move |files: Vec<FileData>| {
        let d = dest.read().clone();
        if d.as_os_str().is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = files
            .into_iter()
            .map(|f| f.path())
            .filter(|p| {
                matches!(
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_ascii_lowercase())
                        .as_deref(),
                    // JPEG / common
                    Some("jpg") | Some("jpeg") | Some("png") | Some("webp") |
                    Some("heic") | Some("heif") | Some("tiff") | Some("tif") |
                    Some("bmp") | Some("avif") |
                    // RAW — Fuji, Sony, Canon, Nikon, DNG, Olympus, Panasonic, Pentax, Leica
                    Some("raf") | Some("arw") | Some("cr2") | Some("cr3") |
                    Some("nef") | Some("nrw") | Some("dng") | Some("orf") |
                    Some("rw2") | Some("pef") | Some("rwl") | Some("raw")
                )
            })
            .collect();

        if paths.is_empty() {
            return;
        }

        let m = *mode.read();
        let total = paths.len();
        progress_done.set(0);
        progress_total.set(total);
        phase.set(Phase::Sorting);

        spawn(async move {
            let (tx, rx) = crossbeam_channel::unbounded::<SortEvent>();
            let dest_clone = d.clone();
            std::thread::spawn(move || {
                sorter::sort_photos(paths, dest_clone, m, tx);
            });

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                let mut finished = false;
                while let Ok(evt) = rx.try_recv() {
                    match evt {
                        SortEvent::Progress { done, total } => {
                            progress_done.set(done);
                            progress_total.set(total);
                        }
                        SortEvent::Done { ok, errors, .. } => {
                            result_ok.set(ok);
                            result_errors.set(errors);
                            phase.set(Phase::Done);
                            finished = true;
                        }
                    }
                }
                if finished {
                    break;
                }
            }
        });
    };

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        main {
            div { id: "top-bar",
                div { id: "dest-row",
                    svg {
                        id: "folder-icon",
                        xmlns: "http://www.w3.org/2000/svg",
                        fill: "none",
                        view_box: "0 0 24 24",
                        stroke_width: "1.5",
                        stroke: "currentColor",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            d: "M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z"
                        }
                    }
                    span { id: "dest-path", title: "{dest_display}", "{dest_display}" }
                    button {
                        id: "choose-btn",
                        onclick: move |_| {
                            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                config::save_dest(&p);
                                dest.set(p);
                            }
                        },
                        "Choose Folder"
                    }
                }
                div { id: "mode-row",
                    span {
                        class: if *mode.read() == SortMode::Copy { "mode-label active" } else { "mode-label" },
                        "Copy"
                    }
                    label { class: "toggle-label",
                        div { class: "toggle",
                            input {
                                class: "toggle-state",
                                r#type: "checkbox",
                                checked: *mode.read() == SortMode::Move,
                                onchange: move |_| mode.set(
                                    if *mode.read() == SortMode::Copy { SortMode::Move } else { SortMode::Copy }
                                ),
                            }
                            div { class: "indicator" }
                        }
                    }
                    span {
                        class: if *mode.read() == SortMode::Move { "mode-label active" } else { "mode-label" },
                        "Move"
                    }
                }
            }

            if *phase.read() == Phase::Idle {
                div {
                    id: "drop-area",
                    style: "background-color: {drop_bg}",
                    ondrag: move |e| e.prevent_default(),
                    ondragenter: move |_| drop_bg.set(HIGHLIGHT.to_string()),
                    ondragleave: move |_| drop_bg.set(BG.to_string()),
                    ondrop: move |e| {
                        drop_bg.set(BG.to_string());
                        e.prevent_default();
                        on_files(e.files());
                    },
                    label { r#for: "file-input", "Drag & Drop photos here" }
                    input {
                        id: "file-input",
                        style: "display:none",
                        r#type: "file",
                        accept: ".jpg,.jpeg,.png,.webp,.heic,.heif,.tiff,.tif,.bmp,.avif,.raf,.arw,.cr2,.cr3,.nef,.nrw,.dng,.orf,.rw2,.pef,.rwl,.raw",
                        multiple: true,
                        onchange: move |e| on_files(e.files()),
                    }
                }
            }

            if *phase.read() == Phase::Sorting {
                div { id: "progress-area",
                    div { id: "progress-label",
                        "{progress_done} / {progress_total} photos"
                    }
                    div { id: "progress-bar-track",
                        div {
                            id: "progress-bar-fill",
                            style: {
                                let pct = if *progress_total.read() > 0 {
                                    (*progress_done.read() as f64 / *progress_total.read() as f64 * 100.0) as u32
                                } else { 0 };
                                format!("width: {}%", pct)
                            }
                        }
                    }
                }
            }

            if *phase.read() == Phase::Done {
                div { id: "done-area",
                    p { id: "done-msg",
                        if *result_errors.read() > 0 {
                            "{result_ok} sorted, {result_errors} errors"
                        } else {
                            "{result_ok} photos sorted"
                        }
                    }
                    button {
                        id: "again-btn",
                        onclick: move |_| {
                            phase.set(Phase::Idle);
                            progress_done.set(0);
                            progress_total.set(0);
                        },
                        "Sort More"
                    }
                }
            }
        }
    }
}

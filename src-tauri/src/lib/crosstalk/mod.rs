// Path: src-tauri/src/lib/crosstalk/mod.rs
// Description: Overview visibility and lifecycle of the read-only Crosstalk companion

mod feed;
mod observer;
mod placement;

use crate::obs::logging;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const LABEL: &str = "crosstalk";

#[derive(Clone, Serialize)]
pub struct Presentation {
    active: bool,
    theme: String,
    repos: Vec<RepoStyle>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct RepoStyle {
    path: String,
    label: String,
    accent: String,
}

pub struct ViewState {
    presentation: Presentation,
    ready: bool,
    height: f64,
    window_visible: bool,
}

impl ViewState {
    fn effective_presentation(&self) -> Presentation {
        let mut presentation = self.presentation.clone();
        presentation.active &= self.window_visible;
        presentation
    }
}

pub struct CrosstalkState {
    view: Mutex<ViewState>,
    observer: Mutex<Option<observer::Observer>>,
    layout_pending: AtomicBool,
    layout: Mutex<()>,
}

impl Default for CrosstalkState {
    fn default() -> Self {
        Self {
            view: Mutex::new(ViewState {
                presentation: Presentation {
                    active: false,
                    theme: "dark".into(),
                    repos: Vec::new(),
                },
                ready: false,
                height: 0.0,
                window_visible: false,
            }),
            observer: Mutex::new(None),
            layout_pending: AtomicBool::new(false),
            layout: Mutex::new(()),
        }
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let main = app
        .get_webview_window("main")
        .ok_or(tauri::Error::WindowNotFound)?;
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("crosstalk.html".into()))
        .title("Intermediary Crosstalk")
        .inner_size(356.0, 1.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .parent(&main)?
        .build()?;
    Ok(())
}

fn require_window(window: &WebviewWindow, label: &str) -> Result<(), String> {
    if window.label() == label {
        Ok(())
    } else {
        Err("Crosstalk caller is not authorized".into())
    }
}

#[tauri::command]
pub async fn crosstalk_configure(
    app: AppHandle,
    window: WebviewWindow,
    active: bool,
    theme: String,
    repos: Vec<RepoStyle>,
) -> Result<(), String> {
    require_window(&window, "main")?;
    if !matches!(theme.as_str(), "dark" | "light" | "warm") {
        return Err("Invalid Crosstalk theme".into());
    }
    let state = app.state::<CrosstalkState>();
    {
        let mut view = state
            .view
            .lock()
            .map_err(|_| "Crosstalk view lock failed")?;
        view.presentation = Presentation {
            active,
            theme,
            repos,
        };
        if !active {
            view.height = 0.0;
        }
        app.emit_to(
            LABEL,
            "crosstalk-presentation",
            view.effective_presentation(),
        )
        .map_err(|e| e.to_string())?;
    }
    if active {
        let app_copy = app.clone();
        tauri::async_runtime::spawn_blocking(move || observer::start(&app_copy))
            .await
            .map_err(|e| e.to_string())??;
    }
    placement::layout(&app)
}

#[tauri::command]
pub fn crosstalk_ready(app: AppHandle, window: WebviewWindow) -> Result<Presentation, String> {
    require_window(&window, LABEL)?;
    let state = app.state::<CrosstalkState>();
    let mut view = state
        .view
        .lock()
        .map_err(|_| "Crosstalk view lock failed")?;
    view.ready = true;
    Ok(view.effective_presentation())
}

#[tauri::command]
pub async fn crosstalk_resize(
    app: AppHandle,
    window: WebviewWindow,
    height: f64,
) -> Result<(), String> {
    require_window(&window, LABEL)?;
    if !height.is_finite() || !(0.0..=4096.0).contains(&height) {
        return Err("Invalid Crosstalk height".into());
    }
    app.state::<CrosstalkState>()
        .view
        .lock()
        .map_err(|_| "Crosstalk view lock failed")?
        .height = height;
    placement::layout(&app)
}

pub fn main_window_event(app: &AppHandle, event: &tauri::WindowEvent) {
    if matches!(
        event,
        tauri::WindowEvent::Moved(_)
            | tauri::WindowEvent::Resized(_)
            | tauri::WindowEvent::ScaleFactorChanged { .. }
    ) {
        if app
            .state::<CrosstalkState>()
            .layout_pending
            .swap(true, Ordering::AcqRel)
        {
            return;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            app.state::<CrosstalkState>()
                .layout_pending
                .store(false, Ordering::Release);
            if let Err(error) = placement::layout(&app) {
                report(&error);
            }
        });
    }
}

pub fn shutdown(app: &AppHandle) {
    if let Ok(mut owner) = app.state::<CrosstalkState>().observer.lock() {
        if let Some(observer) = owner.take() {
            observer.stop();
        }
    }
}

pub fn report(error: &str) {
    logging::log("warn", "crosstalk", "observer", error);
}

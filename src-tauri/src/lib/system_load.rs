// Path: src-tauri/src/lib/system_load.rs
// Description: Hosts the lazily created system-load Sampler and the sample_system_load command

use std::sync::{Arc, Mutex, PoisonError};

use im_system_load::{Sampler, SystemLoadSnapshot};

/// One Sampler, created on the first request; rates span the gap between requests.
#[derive(Default)]
pub struct SystemLoadState {
    sampler: Arc<Mutex<Option<Sampler>>>,
}

#[tauri::command]
pub async fn sample_system_load(
    state: tauri::State<'_, SystemLoadState>,
) -> Result<SystemLoadSnapshot, String> {
    let sampler = Arc::clone(&state.sampler);
    tauri::async_runtime::spawn_blocking(move || sample(&sampler))
        .await
        .map_err(|err| format!("System load sampling task failed: {err}"))?
}

fn sample(slot: &Mutex<Option<Sampler>>) -> Result<SystemLoadSnapshot, String> {
    let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if slot.is_none() {
        let sampler =
            Sampler::new().map_err(|err| format!("System load sampler failed to start: {err}"))?;
        *slot = Some(sampler);
    }
    match slot.as_mut() {
        Some(sampler) => sampler
            .sample()
            .map_err(|err| format!("System load sample failed: {err}")),
        None => Err("System load sampler is unavailable".to_string()),
    }
}

// Only checkpoint atomic overworld states; battles and scripts retain the last
// safe checkpoint rather than saving presentation state the loader cannot restore.
fn save_browser_checkpoint(runtime: &mut BevyRuntimeShell) -> Result<bool> {
    let Some(path) = runtime.quick_save_path.as_ref() else { return Ok(false) };
    let snapshot = runtime.shell.snapshot()?;
    if snapshot.trainer.player_name.is_empty()
        || !visible_quick_save_blockers(runtime, &snapshot, false, false, false).is_empty()
    {
        return Ok(false);
    }
    runtime.shell.save(path)?;
    Ok(true)
}

#[cfg(target_arch = "wasm32")]
fn autosave_browser_progress(mut runtime: ResMut<BevyRuntimeShell>, time: Res<Time>, mut elapsed: Local<f32>) {
    *elapsed += time.delta_seconds();
    if *elapsed < 1.0 { return; }
    *elapsed = 0.0;
    if let Err(error) = save_browser_checkpoint(&mut runtime) {
        bevy::log::error!("browser autosave failed: {error:#}");
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static BROWSER_MUTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static BROWSER_MASTER_GAIN: std::cell::RefCell<Option<web_sys::GainNode>> = const { std::cell::RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn crystal_set_muted(muted: bool) {
    BROWSER_MUTED.with(|value| value.set(muted));
    BROWSER_MASTER_GAIN.with(|gain| {
        if let Some(gain) = gain.borrow().as_ref() {
            gain.gain().set_value(if muted { 0.0 } else { 1.0 });
        }
    });
}

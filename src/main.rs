#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut app = xeh_playground::TemplateApp::default();
    if let Some(path) = std::env::args().skip(1).next() {
        xeh::file::fs_overlay::load_binary(&mut app.xs, path.as_str()).unwrap();
    }
    let mut native_options = eframe::NativeOptions::default();
    native_options.drag_and_drop_support = true;
    eframe::run_native(Box::new(app), native_options);
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(xeh_playground::TemplateApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}

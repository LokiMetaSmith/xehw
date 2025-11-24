use std::path::{Path, PathBuf};
use std::fs;
use eframe::egui;

pub struct FileExplorer {
    root_path: PathBuf,
    pub is_open: bool,
}

impl Default for FileExplorer {
    fn default() -> Self {
        Self {
            root_path: std::env::current_dir().unwrap_or_default(),
            is_open: true,
        }
    }
}

impl FileExplorer {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        if !self.is_open {
            return None;
        }

        let mut selected_file = None;

        ui.heading("Files");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.render_directory(ui, &self.root_path.clone(), &mut selected_file);
        });

        selected_file
    }

    fn render_directory(&self, ui: &mut egui::Ui, path: &Path, selected: &mut Option<PathBuf>) {
        if let Ok(entries) = fs::read_dir(path) {
            let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
            // Sort directories first, then files
            entries.sort_by_key(|e| {
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                (!is_dir, e.file_name())
            });

            for entry in entries {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files/dirs for now
                if name.starts_with('.') && name != "." && name != ".." {
                    continue;
                }

                if path.is_dir() {
                    egui::CollapsingHeader::new(format!("📁 {}", name))
                        .id_salt(path.to_string_lossy().to_string())
                        .show(ui, |ui| {
                            self.render_directory(ui, &path, selected);
                        });
                } else {
                    if ui.button(format!("📄 {}", name)).clicked() {
                        *selected = Some(path);
                    }
                }
            }
        }
    }
}

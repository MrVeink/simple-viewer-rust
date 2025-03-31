mod data_types;
mod data_handler;

use data_handler::{load_csv_file, load_google_sheet};
use data_types::{TableData, DataSource};
use eframe::{egui, Frame, App, CreationContext};
use rfd::FileDialog;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const VERSION: &str = "1.0.0";
const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

struct ScoreViewer {
    data_source: Option<DataSource>,
    data: Option<TableData>,
    theme_is_dark: bool,
    file_path: Option<PathBuf>,
    sheet_url: String,
    sheet_name: String,
    show_cloud_dialog: bool,
    last_update: Instant,
    temp_url: String,
    temp_sheet: String,
}

impl Default for ScoreViewer {
    fn default() -> Self {
        Self {
            data_source: None,
            data: None,
            theme_is_dark: true,
            file_path: None,
            sheet_url: String::new(),
            sheet_name: String::new(),
            show_cloud_dialog: false,
            last_update: Instant::now(),
            temp_url: String::new(),
            temp_sheet: String::new(),
        }
    }
}

impl App for ScoreViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let now = Instant::now();
        
        // Auto-refresh data
        if now.duration_since(self.last_update) >= UPDATE_INTERVAL {
            self.last_update = now;
            self.refresh_data();
        }
        
        // Apply theme
        ctx.set_visuals(if self.theme_is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        // Top bar menu
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open CSV...").clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Connect to Google Sheet...").clicked() {
                        self.show_cloud_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button(if self.theme_is_dark { "Light Theme" } else { "Dark Theme" }).clicked() {
                        self.theme_is_dark = !self.theme_is_dark;
                        ui.close_menu();
                    }
                    if ui.button("Refresh Data").clicked() {
                        self.refresh_data();
                        ui.close_menu();
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Score Viewer v{}", VERSION));
                });
            });
        });
        
        // Status bar
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match &self.data_source {
                    Some(DataSource::Local(path)) => {
                        ui.label(format!("Local file: {}", path.display()));
                    },
                    Some(DataSource::Cloud(url, sheet)) => {
                        ui.label(format!("Google Sheet: {} ({})", url, sheet));
                    },
                    None => {
                        ui.label("No data source selected");
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(data) = &self.data {
                        ui.label(format!("Rows: {}", data.rows.len()));
                    }
                });
            });
        });
        
        // Main content area with table
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(data) = &self.data {
                self.display_table(ui, data);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No data loaded. Please select a local file or connect to Google Sheets.");
                });
            }
        });
        
        // Cloud connection dialog
        if self.show_cloud_dialog {
            egui::Window::new("Connect to Google Sheet")
                .fixed_size([400.0, 200.0])
                .show(ctx, |ui| {
                    ui.label("Google Sheet URL:");
                    ui.text_edit_singleline(&mut self.temp_url);
                    ui.label("Sheet Name (optional):");
                    ui.text_edit_singleline(&mut self.temp_sheet);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Connect").clicked() {
                            self.connect_to_sheet();
                            self.show_cloud_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_cloud_dialog = false;
                        }
                    });
                });
        }
    }
}

impl ScoreViewer {
    fn refresh_data(&mut self) {
        match &self.data_source {
            Some(DataSource::Local(path)) => {
                if let Ok(data) = load_csv_file(path) {
                    self.data = Some(data);
                }
            },
            Some(DataSource::Cloud(url, sheet)) => {
                if let Ok(data) = load_google_sheet(url, sheet) {
                    self.data = Some(data);
                }
            },
            None => {}
        }
    }
    
    fn open_file_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("CSV Files", &["csv"])
            .pick_file() {
            
            self.file_path = Some(path.clone());
            self.data_source = Some(DataSource::Local(path.clone()));
            
            if let Ok(data) = load_csv_file(&path) {
                self.data = Some(data);
            }
        }
    }
    
    fn connect_to_sheet(&mut self) {
        if !self.temp_url.is_empty() {
            self.sheet_url = self.temp_url.clone();
            self.sheet_name = self.temp_sheet.clone();
            self.data_source = Some(DataSource::Cloud(
                self.sheet_url.clone(),
                self.sheet_name.clone()
            ));
            
            if let Ok(data) = load_google_sheet(&self.sheet_url, &self.sheet_name) {
                self.data = Some(data);
            }
        }
    }
    
    fn display_table(&self, ui: &mut egui::Ui, data: &TableData) {
        egui::ScrollArea::both().show(ui, |ui| {
            // Table with headers and data rows
            egui::Grid::new("data_grid")
                .striped(true)
                .show(ui, |ui| {
                    // Headers
                    for header in &data.headers {
                        ui.strong(header);
                    }
                    ui.end_row();
                    
                    // Data rows
                    for row in &data.rows {
                        for cell in row {
                            ui.label(cell);
                        }
                        ui.end_row();
                    }
                });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 768.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "Score Viewer",
        options,
        Box::new(|_cc: &CreationContext| Box::new(ScoreViewer::default()))
    )
}

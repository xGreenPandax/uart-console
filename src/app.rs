use chrono::Local;
use egui::RichText;
use egui_extras::{Column, TableBuilder};
use regex::Regex;

use crate::serial_port::{SerialEvent, SerialPortManager};
use crate::settings::{Settings, SettingsWindow};

/// One parsed data row
#[derive(Clone)]
struct DataRow {
    timestamp: String,
    raw: String,
    columns: Vec<String>,
    matched: bool,
}

pub struct UartConsoleApp {
    settings: Settings,
    settings_win: SettingsWindow,
    serial: SerialPortManager,
    rows: Vec<DataRow>,
    raw_log: Vec<String>,
    compiled_regex: Option<Regex>,
    send_input: String,
    auto_scroll: bool,
    show_raw: bool,
    status_msg: String,
    status_is_error: bool,
    /// Number of capture groups (columns) from the current regex
    num_columns: usize,
}

impl UartConsoleApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        let settings_win = SettingsWindow::new(&settings);
        let mut app = Self {
            settings_win,
            serial: SerialPortManager::new(),
            rows: Vec::new(),
            raw_log: Vec::new(),
            compiled_regex: None,
            send_input: String::new(),
            auto_scroll: true,
            show_raw: false,
            status_msg: "Disconnected".to_string(),
            status_is_error: false,
            num_columns: 0,
            settings: Settings::default(),
        };
        app.apply_settings(settings);
        app
    }

    fn apply_settings(&mut self, settings: Settings) {
        self.settings = settings;
        self.compile_regex();
    }

    fn compile_regex(&mut self) {
        let pattern = &self.settings.regex_pattern;
        if pattern.is_empty() {
            self.compiled_regex = None;
            self.num_columns = 0;
        } else {
            match Regex::new(pattern) {
                Ok(re) => {
                    self.num_columns = re.captures_len().saturating_sub(1);
                    self.compiled_regex = Some(re);
                }
                Err(e) => {
                    self.set_error(format!("Regex error: {}", e));
                    self.compiled_regex = None;
                    self.num_columns = 0;
                }
            }
        }
        // re-parse existing raw lines
        self.reparse_all();
    }

    fn reparse_all(&mut self) {
        let raws: Vec<String> = self.rows.iter().map(|r| r.raw.clone()).collect();
        self.rows = raws.iter().map(|raw| self.parse_line(raw)).collect();
    }

    fn parse_line(&self, line: &str) -> DataRow {
        let timestamp = if self.settings.show_timestamp {
            Local::now().format("%H:%M:%S%.3f").to_string()
        } else {
            String::new()
        };

        let (columns, matched) = if let Some(re) = &self.compiled_regex {
            if let Some(caps) = re.captures(line) {
                let cols: Vec<String> = (1..caps.len())
                    .map(|i| caps.get(i).map_or("", |m| m.as_str()).to_string())
                    .collect();
                (cols, true)
            } else {
                (vec!["<no match>".to_string()], false)
            }
        } else {
            (vec![line.to_string()], true)
        };

        DataRow {
            timestamp,
            raw: line.to_string(),
            columns,
            matched,
        }
    }

    fn ingest_line(&mut self, line: String) {
        self.raw_log.push(line.clone());
        let row = self.parse_line(&line);
        self.rows.push(row);

        let max = self.settings.max_rows;
        if self.rows.len() > max {
            let drain = self.rows.len() - max;
            self.rows.drain(..drain);
        }
        if self.raw_log.len() > max {
            let drain = self.raw_log.len() - max;
            self.raw_log.drain(..drain);
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
        self.status_is_error = false;
    }

    fn set_error(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
        self.status_is_error = true;
    }

    fn connect(&mut self) {
        match self.serial.connect(&self.settings) {
            Ok(()) => self.set_status("Connecting..."),
            Err(e) => self.set_error(e),
        }
    }

    fn disconnect(&mut self) {
        self.serial.disconnect();
        self.set_status("Disconnected");
    }

    fn send_input(&mut self) {
        if self.send_input.is_empty() {
            return;
        }
        let mut data = self.send_input.as_bytes().to_vec();
        data.extend_from_slice(self.settings.tx_line_ending.as_bytes());
        self.serial.send(data);
        self.send_input.clear();
    }

    fn clear_data(&mut self) {
        self.rows.clear();
        self.raw_log.clear();
    }

    fn column_header(&self, idx: usize) -> String {
        let names = self.settings.column_names_list();
        if idx < names.len() {
            names[idx].clone()
        } else {
            // Try named capture groups
            if let Some(re) = &self.compiled_regex {
                let capture_names: Vec<Option<&str>> = re.capture_names().collect();
                // capture_names[0] = None (whole match), [1..] = groups
                if idx + 1 < capture_names.len() {
                    if let Some(name) = capture_names[idx + 1] {
                        return name.to_string();
                    }
                }
            }
            format!("Col {}", idx + 1)
        }
    }

    fn export_csv(&self) {
        use std::io::Write;
        let path = format!(
            "uart_export_{}.csv",
            Local::now().format("%Y%m%d_%H%M%S")
        );
        if let Ok(mut file) = std::fs::File::create(&path) {
            // header
            let mut header = if self.settings.show_timestamp {
                vec!["Timestamp".to_string()]
            } else {
                vec![]
            };
            if self.compiled_regex.is_some() {
                for i in 0..self.num_columns {
                    header.push(self.column_header(i));
                }
            } else {
                header.push("Data".to_string());
            }
            let _ = writeln!(file, "{}", header.join(","));

            for row in &self.rows {
                let mut cells: Vec<String> = if self.settings.show_timestamp {
                    vec![row.timestamp.clone()]
                } else {
                    vec![]
                };
                cells.extend(row.columns.iter().cloned());
                let _ = writeln!(file, "{}", cells.join(","));
            }
        }
    }

    fn poll_serial_events(&mut self) {
        let events = self.serial.poll_events();
        for ev in events {
            match ev {
                SerialEvent::Connected => {
                    self.serial.is_connected = true;
                    self.set_status(format!(
                        "Connected to {} @ {} baud",
                        self.settings.port_name, self.settings.baud_rate
                    ));
                }
                SerialEvent::Disconnected => {
                    self.serial.is_connected = false;
                    self.set_status("Disconnected");
                }
                SerialEvent::Data(line) => {
                    self.ingest_line(line);
                }
                SerialEvent::Error(e) => {
                    self.serial.is_connected = false;
                    self.set_error(e);
                }
            }
        }
    }

    // --- UI rendering ---

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let connected = self.serial.is_connected;

            // Connect / Disconnect
            if connected {
                if ui
                    .add_sized(
                        [110.0, 28.0],
                        egui::Button::new(
                            RichText::new("Disconnect").color(egui::Color32::from_rgb(255, 80, 80)),
                        ),
                    )
                    .clicked()
                {
                    self.disconnect();
                }
            } else {
                if ui
                    .add_sized(
                        [110.0, 28.0],
                        egui::Button::new(
                            RichText::new("Connect").color(egui::Color32::from_rgb(80, 200, 80)),
                        ),
                    )
                    .clicked()
                {
                    self.connect();
                }
            }

            ui.separator();

            // Settings
            if ui
                .add_sized([90.0, 28.0], egui::Button::new("Settings"))
                .clicked()
            {
                self.settings_win.open(&self.settings);
            }

            ui.separator();

            // Regex pattern (quick edit in toolbar)
            ui.label("Regex:");
            let re_resp = ui.add(
                egui::TextEdit::singleline(&mut self.settings.regex_pattern)
                    .hint_text("(group1)(group2)...")
                    .desired_width(280.0),
            );
            if re_resp.lost_focus() || re_resp.changed() {
                let new_settings = self.settings.clone();
                self.apply_settings(new_settings);
            }

            ui.separator();

            // Auto-scroll toggle
            ui.checkbox(&mut self.auto_scroll, "Auto-scroll");

            // Show raw toggle
            ui.checkbox(&mut self.show_raw, "Raw view");

            ui.separator();

            // Clear
            if ui
                .add_sized([60.0, 28.0], egui::Button::new("Clear"))
                .clicked()
            {
                self.clear_data();
            }

            // Export
            if ui
                .add_sized([90.0, 28.0], egui::Button::new("Export CSV"))
                .clicked()
            {
                let path = format!("uart_export_{}.csv", Local::now().format("%Y%m%d_%H%M%S"));
                self.export_csv();
                self.set_status(format!("Exported to {}", path));
            }
        });
    }

    fn render_data_table(&mut self, ui: &mut egui::Ui) {
        let show_ts = self.settings.show_timestamp;
        let has_regex = self.compiled_regex.is_some();
        let num_cols = self.num_columns;
        let default_text_color = ui.visuals().text_color();

        // Build column layout
        let mut builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .auto_shrink(false);

        if self.auto_scroll {
            builder = builder.scroll_to_row(self.rows.len().saturating_sub(1), Some(egui::Align::BOTTOM));
        }

        // Timestamp column
        if show_ts {
            builder = builder.column(Column::initial(110.0).at_least(80.0).resizable(true));
        }

        if has_regex && num_cols > 0 {
            for _ in 0..num_cols {
                builder = builder.column(Column::initial(120.0).at_least(60.0).resizable(true));
            }
        } else {
            // Raw data column
            builder = builder.column(Column::remainder().at_least(100.0));
        }

        let table = builder.header(22.0, |mut header| {
            if show_ts {
                header.col(|ui| {
                    ui.strong("Timestamp");
                });
            }
            if has_regex && num_cols > 0 {
                for i in 0..num_cols {
                    header.col(|ui| {
                        ui.strong(self.column_header(i));
                    });
                }
            } else {
                header.col(|ui| {
                    ui.strong("Data");
                });
            }
        });

        // Row count for borrow
        let rows_len = self.rows.len();

        table.body(|body| {
            body.rows(18.0, rows_len, |mut row_widget| {
                let idx = row_widget.index();
                if idx >= self.rows.len() {
                    return;
                }
                let row = &self.rows[idx];
                let color = if !row.matched {
                    egui::Color32::from_rgb(160, 100, 100)
                } else {
                    default_text_color
                };

                if show_ts {
                    row_widget.col(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(140, 140, 200), &row.timestamp);
                    });
                }

                if has_regex && num_cols > 0 {
                    for col_i in 0..num_cols {
                        row_widget.col(|ui| {
                            let val = row.columns.get(col_i).map(String::as_str).unwrap_or("");
                            ui.colored_label(color, val);
                        });
                    }
                } else {
                    row_widget.col(|ui| {
                        let val = row.columns.first().map(String::as_str).unwrap_or(&row.raw);
                        ui.colored_label(color, val);
                    });
                }
            });
        });
    }

    fn render_raw_log(&mut self, ui: &mut egui::Ui) {
        let scroll = egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(self.auto_scroll);

        scroll.show(ui, |ui| {
            let font_id = egui::FontId::monospace(12.0);
            for line in &self.raw_log {
                ui.label(RichText::new(line).font(font_id.clone()).color(
                    egui::Color32::from_rgb(180, 220, 180),
                ));
            }
        });
    }

    fn render_send_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Send:");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.send_input)
                    .desired_width(ui.available_width() - 90.0)
                    .hint_text("type data to send..."),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.send_input();
            }
            if ui
                .add_sized([80.0, 24.0], egui::Button::new("Send"))
                .clicked()
            {
                self.send_input();
            }
        });
    }

    fn render_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Connection indicator
            let (dot_color, dot_label) = if self.serial.is_connected {
                (egui::Color32::from_rgb(60, 200, 60), "  Connected  ")
            } else {
                (egui::Color32::from_rgb(180, 60, 60), "  Disconnected  ")
            };

            ui.colored_label(dot_color, dot_label);
            ui.separator();

            let msg_color = if self.status_is_error {
                egui::Color32::from_rgb(255, 100, 100)
            } else {
                ui.visuals().text_color()
            };
            ui.colored_label(msg_color, &self.status_msg);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Rows: {}", self.rows.len()));
            });
        });
    }
}

impl eframe::App for UartConsoleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll serial events every frame
        self.poll_serial_events();

        // Request repaint while connected (for live data)
        if self.serial.is_connected {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }

        // Handle settings window result
        if let Some(new_settings) = self.settings_win.show(ctx) {
            new_settings.save();
            let needs_reconnect = self.serial.is_connected
                && (new_settings.port_name != self.settings.port_name
                    || new_settings.baud_rate != self.settings.baud_rate);
            self.apply_settings(new_settings);
            if needs_reconnect {
                self.connect();
            }
        }

        // Top panel: toolbar
        egui::TopBottomPanel::top("toolbar")
            .min_height(36.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.render_toolbar(ui);
                ui.add_space(2.0);
            });

        // Bottom panels
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(22.0)
            .show(ctx, |ui| {
                self.render_status_bar(ui);
            });

        egui::TopBottomPanel::bottom("send_bar")
            .min_height(32.0)
            .show(ctx, |ui| {
                ui.add_space(3.0);
                self.render_send_bar(ui);
                ui.add_space(3.0);
            });

        // Central: data view
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_raw {
                self.render_raw_log(ui);
            } else {
                self.render_data_table(ui);
            }
        });
    }
}

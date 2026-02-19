use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppDataBits {
    Five,
    Six,
    Seven,
    Eight,
}

impl AppDataBits {
    pub fn to_serial(&self) -> serialport::DataBits {
        match self {
            AppDataBits::Five => serialport::DataBits::Five,
            AppDataBits::Six => serialport::DataBits::Six,
            AppDataBits::Seven => serialport::DataBits::Seven,
            AppDataBits::Eight => serialport::DataBits::Eight,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            AppDataBits::Five => "5",
            AppDataBits::Six => "6",
            AppDataBits::Seven => "7",
            AppDataBits::Eight => "8",
        }
    }
    pub fn all() -> &'static [AppDataBits] {
        &[AppDataBits::Five, AppDataBits::Six, AppDataBits::Seven, AppDataBits::Eight]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppStopBits {
    One,
    Two,
}

impl AppStopBits {
    pub fn to_serial(&self) -> serialport::StopBits {
        match self {
            AppStopBits::One => serialport::StopBits::One,
            AppStopBits::Two => serialport::StopBits::Two,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            AppStopBits::One => "1",
            AppStopBits::Two => "2",
        }
    }
    pub fn all() -> &'static [AppStopBits] {
        &[AppStopBits::One, AppStopBits::Two]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppParity {
    None,
    Odd,
    Even,
}

impl AppParity {
    pub fn to_serial(&self) -> serialport::Parity {
        match self {
            AppParity::None => serialport::Parity::None,
            AppParity::Odd => serialport::Parity::Odd,
            AppParity::Even => serialport::Parity::Even,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            AppParity::None => "None",
            AppParity::Odd => "Odd",
            AppParity::Even => "Even",
        }
    }
    pub fn all() -> &'static [AppParity] {
        &[AppParity::None, AppParity::Odd, AppParity::Even]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppFlowControl {
    None,
    Software,
    Hardware,
}

impl AppFlowControl {
    pub fn to_serial(&self) -> serialport::FlowControl {
        match self {
            AppFlowControl::None => serialport::FlowControl::None,
            AppFlowControl::Software => serialport::FlowControl::Software,
            AppFlowControl::Hardware => serialport::FlowControl::Hardware,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            AppFlowControl::None => "None",
            AppFlowControl::Software => "XON/XOFF",
            AppFlowControl::Hardware => "RTS/CTS",
        }
    }
    pub fn all() -> &'static [AppFlowControl] {
        &[AppFlowControl::None, AppFlowControl::Software, AppFlowControl::Hardware]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LineEnding {
    None,
    CR,
    LF,
    CrLf,
}

impl LineEnding {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            LineEnding::None => b"",
            LineEnding::CR => b"\r",
            LineEnding::LF => b"\n",
            LineEnding::CrLf => b"\r\n",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            LineEnding::None => "None",
            LineEnding::CR => "CR (\\r)",
            LineEnding::LF => "LF (\\n)",
            LineEnding::CrLf => "CRLF (\\r\\n)",
        }
    }
    pub fn all() -> &'static [LineEnding] {
        &[LineEnding::None, LineEnding::CR, LineEnding::LF, LineEnding::CrLf]
    }
}

pub const BAUD_RATES: &[u32] = &[
    300, 600, 1200, 2400, 4800, 9600, 14400, 19200, 38400, 57600, 115200, 230400, 460800, 921600,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: AppDataBits,
    pub stop_bits: AppStopBits,
    pub parity: AppParity,
    pub flow_control: AppFlowControl,
    pub regex_pattern: String,
    pub column_names: String,
    pub max_rows: usize,
    pub show_timestamp: bool,
    pub rx_line_ending: LineEnding,
    pub tx_line_ending: LineEnding,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            data_bits: AppDataBits::Eight,
            stop_bits: AppStopBits::One,
            parity: AppParity::None,
            flow_control: AppFlowControl::None,
            regex_pattern: String::new(),
            column_names: String::new(),
            max_rows: 2000,
            show_timestamp: true,
            rx_line_ending: LineEnding::LF,
            tx_line_ending: LineEnding::CrLf,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, data);
        }
    }

    fn config_path() -> std::path::PathBuf {
        let mut path = std::env::current_exe().unwrap_or_default();
        path.pop();
        path.push("uart_console_settings.json");
        path
    }

    pub fn column_names_list(&self) -> Vec<String> {
        if self.column_names.trim().is_empty() {
            vec![]
        } else {
            self.column_names
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        }
    }
}

// Settings window UI state
pub struct SettingsWindow {
    pub edit: Settings,
    pub is_open: bool,
    pub available_ports: Vec<String>,
    pub test_input: String,
    pub test_result: String,
    pub regex_error: String,
    pub custom_baud: String,
    pub show_custom_baud: bool,
}

impl SettingsWindow {
    pub fn new(settings: &Settings) -> Self {
        Self {
            edit: settings.clone(),
            is_open: false,
            available_ports: vec![],
            test_input: String::new(),
            test_result: String::new(),
            regex_error: String::new(),
            custom_baud: String::new(),
            show_custom_baud: false,
        }
    }

    pub fn open(&mut self, settings: &Settings) {
        self.edit = settings.clone();
        self.is_open = true;
        self.refresh_ports();
        self.regex_error.clear();
        self.test_result.clear();
    }

    pub fn refresh_ports(&mut self) {
        self.available_ports = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect();
    }

    pub fn validate_regex(&mut self) {
        if self.edit.regex_pattern.is_empty() {
            self.regex_error.clear();
            self.test_result.clear();
            return;
        }
        match regex::Regex::new(&self.edit.regex_pattern) {
            Ok(re) => {
                self.regex_error.clear();
                if !self.test_input.is_empty() {
                    if let Some(caps) = re.captures(&self.test_input) {
                        let groups: Vec<String> = (1..caps.len())
                            .map(|i| caps.get(i).map_or("", |m| m.as_str()).to_string())
                            .collect();
                        self.test_result = format!("Match: [{}]", groups.join("] ["));
                    } else {
                        self.test_result = "No match".to_string();
                    }
                }
            }
            Err(e) => {
                self.regex_error = format!("Regex error: {}", e);
                self.test_result.clear();
            }
        }
    }

    /// Renders the settings window. Returns Some(Settings) if Apply was clicked.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Settings> {
        if !self.is_open {
            return None;
        }

        let mut result = None;
        let mut open = self.is_open;

        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_width(440.0)
            .min_width(380.0)
            .show(ctx, |ui| {
                result = self.render_content(ui);
            });

        self.is_open = open;
        result
    }

    fn render_content(&mut self, ui: &mut egui::Ui) -> Option<Settings> {
        let mut result = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            // --- Connection ---
            ui.heading("Connection");
            ui.separator();

            egui::Grid::new("conn_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    // Port
                    ui.label("Port:");
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("port_combo")
                            .selected_text(if self.edit.port_name.is_empty() {
                                "-- select --"
                            } else {
                                &self.edit.port_name
                            })
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for port in &self.available_ports.clone() {
                                    ui.selectable_value(
                                        &mut self.edit.port_name,
                                        port.clone(),
                                        port,
                                    );
                                }
                            });
                        if ui.button("Refresh").clicked() {
                            self.refresh_ports();
                        }
                    });
                    ui.end_row();

                    // Baud rate
                    ui.label("Baud Rate:");
                    ui.horizontal(|ui| {
                        let baud_label = if BAUD_RATES.contains(&self.edit.baud_rate) {
                            self.edit.baud_rate.to_string()
                        } else {
                            format!("{} (custom)", self.edit.baud_rate)
                        };
                        egui::ComboBox::from_id_salt("baud_combo")
                            .selected_text(baud_label)
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for &baud in BAUD_RATES {
                                    ui.selectable_value(
                                        &mut self.edit.baud_rate,
                                        baud,
                                        baud.to_string(),
                                    );
                                }
                                ui.selectable_value(
                                    &mut self.show_custom_baud,
                                    true,
                                    "Custom...",
                                );
                            });
                        if self.show_custom_baud {
                            ui.text_edit_singleline(&mut self.custom_baud);
                            if ui.button("Set").clicked() {
                                if let Ok(baud) = self.custom_baud.parse::<u32>() {
                                    if baud > 0 {
                                        self.edit.baud_rate = baud;
                                        self.show_custom_baud = false;
                                    }
                                }
                            }
                        }
                    });
                    ui.end_row();

                    // Data bits
                    ui.label("Data Bits:");
                    egui::ComboBox::from_id_salt("data_bits_combo")
                        .selected_text(self.edit.data_bits.label())
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for bits in AppDataBits::all() {
                                ui.selectable_value(
                                    &mut self.edit.data_bits,
                                    bits.clone(),
                                    bits.label(),
                                );
                            }
                        });
                    ui.end_row();

                    // Stop bits
                    ui.label("Stop Bits:");
                    egui::ComboBox::from_id_salt("stop_bits_combo")
                        .selected_text(self.edit.stop_bits.label())
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for bits in AppStopBits::all() {
                                ui.selectable_value(
                                    &mut self.edit.stop_bits,
                                    bits.clone(),
                                    bits.label(),
                                );
                            }
                        });
                    ui.end_row();

                    // Parity
                    ui.label("Parity:");
                    egui::ComboBox::from_id_salt("parity_combo")
                        .selected_text(self.edit.parity.label())
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for p in AppParity::all() {
                                ui.selectable_value(
                                    &mut self.edit.parity,
                                    p.clone(),
                                    p.label(),
                                );
                            }
                        });
                    ui.end_row();

                    // Flow control
                    ui.label("Flow Control:");
                    egui::ComboBox::from_id_salt("flow_combo")
                        .selected_text(self.edit.flow_control.label())
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for fc in AppFlowControl::all() {
                                ui.selectable_value(
                                    &mut self.edit.flow_control,
                                    fc.clone(),
                                    fc.label(),
                                );
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(12.0);
            ui.heading("Regex Parser");
            ui.separator();

            ui.label("Regex pattern (each capture group = one column):");
            let re_changed = ui
                .add(
                    egui::TextEdit::singleline(&mut self.edit.regex_pattern)
                        .hint_text("e.g. T=([-\\d.]+),H=([-\\d.]+),P=([-\\d.]+)")
                        .desired_width(f32::INFINITY),
                )
                .changed();

            if re_changed {
                self.validate_regex();
            }

            if !self.regex_error.is_empty() {
                ui.colored_label(egui::Color32::RED, &self.regex_error.clone());
            }

            ui.add_space(6.0);
            ui.label("Test string:");
            let test_changed = ui
                .add(
                    egui::TextEdit::singleline(&mut self.test_input)
                        .hint_text("Paste a sample UART line here to test")
                        .desired_width(f32::INFINITY),
                )
                .changed();

            if test_changed {
                self.validate_regex();
            }

            if !self.test_result.is_empty() {
                let color = if self.test_result.starts_with("Match") {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::YELLOW
                };
                ui.colored_label(color, &self.test_result.clone());
            }

            ui.add_space(6.0);
            ui.label("Column names (comma-separated, optional):");
            ui.add(
                egui::TextEdit::singleline(&mut self.edit.column_names)
                    .hint_text("Temperature, Humidity, Pressure")
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(12.0);
            ui.heading("Display");
            ui.separator();

            egui::Grid::new("display_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Max rows:");
                    ui.add(egui::DragValue::new(&mut self.edit.max_rows).range(100..=100000));
                    ui.end_row();

                    ui.label("Show timestamp:");
                    ui.checkbox(&mut self.edit.show_timestamp, "");
                    ui.end_row();

                    ui.label("RX line ending:");
                    egui::ComboBox::from_id_salt("rx_le_combo")
                        .selected_text(self.edit.rx_line_ending.label())
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for le in LineEnding::all() {
                                ui.selectable_value(
                                    &mut self.edit.rx_line_ending,
                                    le.clone(),
                                    le.label(),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("TX line ending:");
                    egui::ComboBox::from_id_salt("tx_le_combo")
                        .selected_text(self.edit.tx_line_ending.label())
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for le in LineEnding::all() {
                                ui.selectable_value(
                                    &mut self.edit.tx_line_ending,
                                    le.clone(),
                                    le.label(),
                                );
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(16.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_sized([100.0, 28.0], egui::Button::new("Apply"))
                    .clicked()
                {
                    result = Some(self.edit.clone());
                    self.is_open = false;
                }
                if ui
                    .add_sized([100.0, 28.0], egui::Button::new("Cancel"))
                    .clicked()
                {
                    self.is_open = false;
                }
            });
        });

        result
    }
}

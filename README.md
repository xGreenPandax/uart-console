# UART Console

A cross-platform graphical UART/serial terminal written in Rust, built with [egui](https://github.com/emilk/egui) and [eframe](https://github.com/emilk/egui/tree/master/crates/eframe).

## Key Feature — Live Regex Column Parser

The main distinguishing feature is the **real-time regex-based data splitter**. Enter any regular expression with capture groups, and every incoming UART line is instantly split into columns displayed in a live-updating table.

**Example**

Incoming UART data:
```
T=25.3,H=60.2,P=1013
T=25.4,H=60.1,P=1012
```

Regex pattern:
```
T=([-\d.]+),H=([-\d.]+),P=([-\d.]+)
```

Result — three auto-generated columns:

| Timestamp    | Col 1 | Col 2 | Col 3 |
|--------------|-------|-------|-------|
| 00:00:01.123 | 25.3  | 60.2  | 1013  |
| 00:00:01.456 | 25.4  | 60.1  | 1012  |

Named capture groups (`(?P<temp>[\d.]+)`) are automatically used as column headers.

---

## Features

- **Live table view** — incoming lines are parsed and displayed as table rows in real time
- **Regex column splitting** — any number of columns, defined by capture groups in a single regex
- **Named capture group headers** — `(?P<name>...)` becomes the column title automatically
- **Custom column names** — override headers via comma-separated list in Settings
- **Raw log view** — toggle between parsed table and raw monospace log
- **Send data** — type and send strings to the serial port (Enter or Send button)
- **Configurable line endings** — independent RX and TX line ending (None / CR / LF / CRLF)
- **Timestamp column** — optional HH:MM:SS.mmm prefix for each row
- **Auto-scroll** — table always follows the latest data
- **Export CSV** — export the current table to a timestamped `.csv` file
- **Persistent settings** — connection and regex settings saved to `uart_console_settings.json` next to the executable
- **Unmatched line highlighting** — lines that don't match the regex are shown in red

---

## Settings Window

Click the **Settings** button in the toolbar to open the configuration panel.

| Setting | Description |
|---|---|
| Port | Serial port name (e.g. `COM3`, `/dev/ttyUSB0`) |
| Baud Rate | Standard rates from 300 to 921600, plus custom input |
| Data Bits | 5 / 6 / 7 / 8 |
| Stop Bits | 1 / 2 |
| Parity | None / Odd / Even |
| Flow Control | None / XON-XOFF / RTS-CTS |
| Regex Pattern | Pattern with capture groups for column splitting |
| Test String | Paste a sample line to verify the regex live |
| Column Names | Comma-separated header overrides |
| Max Rows | Maximum number of rows kept in memory (100–100 000) |
| Timestamp | Show/hide the timestamp column |
| RX Line Ending | How incoming data is split into lines |
| TX Line Ending | Appended to every sent string |

---

## Building

**Requirements:**
- Rust 1.85 or newer (uses `image 0.25` dependency)
- On Windows: MSVC toolchain recommended

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run directly
cargo run
```

The release binary is located at `target/release/uart_console.exe` (Windows) or `target/release/uart_console` (Linux/macOS).

---

## Dependencies

| Crate | Purpose |
|---|---|
| `eframe` / `egui` | Immediate-mode GUI framework |
| `egui_extras` | Resizable table widget (`TableBuilder`) |
| `serialport` | Cross-platform serial port access |
| `regex` | Regex engine for line parsing |
| `serde` / `serde_json` | Settings serialization |
| `chrono` | Timestamp formatting |

---

## Usage Tips

- The **Regex** field in the toolbar allows quick edits without opening Settings.
  The table redraws immediately and all existing rows are re-parsed.
- Use **Raw view** when debugging protocol framing — shows unmodified received text.
- **Export CSV** saves to the current working directory with a filename like
  `uart_export_20260220_143512.csv`.
- Settings are saved automatically when you click **Apply** in the Settings window.

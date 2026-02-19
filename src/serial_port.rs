use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::settings::Settings;

pub enum SerialCommand {
    Send(Vec<u8>),
    Disconnect,
}

pub enum SerialEvent {
    Data(String),
    Connected,
    Disconnected,
    Error(String),
}

pub struct SerialPortManager {
    pub cmd_tx: Option<mpsc::Sender<SerialCommand>>,
    pub event_rx: mpsc::Receiver<SerialEvent>,
    event_tx: mpsc::SyncSender<SerialEvent>,
    pub is_connected: bool,
}

impl SerialPortManager {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::sync_channel(256);
        Self {
            cmd_tx: None,
            event_rx,
            event_tx,
            is_connected: false,
        }
    }

    pub fn connect(&mut self, settings: &Settings) -> Result<(), String> {
        if self.is_connected {
            self.disconnect();
        }

        let port_name = settings.port_name.clone();
        if port_name.is_empty() {
            return Err("No port selected".to_string());
        }

        let port = serialport::new(&port_name, settings.baud_rate)
            .data_bits(settings.data_bits.to_serial())
            .stop_bits(settings.stop_bits.to_serial())
            .parity(settings.parity.to_serial())
            .flow_control(settings.flow_control.to_serial())
            .timeout(Duration::from_millis(50))
            .open()
            .map_err(|e| format!("Failed to open {}: {}", port_name, e))?;

        let (cmd_tx, cmd_rx) = mpsc::channel::<SerialCommand>();
        self.cmd_tx = Some(cmd_tx);
        self.is_connected = true;

        let event_tx = self.event_tx.clone();
        let rx_line_ending = settings.rx_line_ending.clone();

        thread::spawn(move || {
            run_serial_thread(port, cmd_rx, event_tx, rx_line_ending);
        });

        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(SerialCommand::Disconnect);
        }
        self.is_connected = false;
    }

    pub fn send(&self, data: Vec<u8>) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(SerialCommand::Send(data));
        }
    }

    /// Drain all pending events; returns them as a vec.
    pub fn poll_events(&self) -> Vec<SerialEvent> {
        let mut events = Vec::new();
        loop {
            match self.event_rx.try_recv() {
                Ok(ev) => events.push(ev),
                Err(_) => break,
            }
        }
        events
    }
}

fn run_serial_thread(
    mut port: Box<dyn serialport::SerialPort>,
    cmd_rx: mpsc::Receiver<SerialCommand>,
    event_tx: mpsc::SyncSender<SerialEvent>,
    rx_line_ending: crate::settings::LineEnding,
) {
    let _ = event_tx.send(SerialEvent::Connected);

    let mut rx_buf = Vec::<u8>::with_capacity(4096);
    let mut read_buf = [0u8; 256];

    loop {
        // Check for commands (non-blocking)
        loop {
            match cmd_rx.try_recv() {
                Ok(SerialCommand::Disconnect) => {
                    let _ = event_tx.send(SerialEvent::Disconnected);
                    return;
                }
                Ok(SerialCommand::Send(data)) => {
                    if let Err(e) = port.write_all(&data) {
                        let _ = event_tx.send(SerialEvent::Error(format!("Write error: {}", e)));
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let _ = event_tx.send(SerialEvent::Disconnected);
                    return;
                }
            }
        }

        // Read from port
        match port.read(&mut read_buf) {
            Ok(0) => {}
            Ok(n) => {
                rx_buf.extend_from_slice(&read_buf[..n]);
                // Extract complete lines
                extract_lines(&mut rx_buf, &rx_line_ending, &event_tx);
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                // Normal timeout - continue
            }
            Err(e) => {
                let _ = event_tx.send(SerialEvent::Error(format!("Read error: {}", e)));
                let _ = event_tx.send(SerialEvent::Disconnected);
                return;
            }
        }
    }
}

fn extract_lines(
    buf: &mut Vec<u8>,
    line_ending: &crate::settings::LineEnding,
    event_tx: &mpsc::SyncSender<SerialEvent>,
) {
    use crate::settings::LineEnding;

    match line_ending {
        LineEnding::None => {
            // Send everything as-is
            if !buf.is_empty() {
                if let Ok(s) = String::from_utf8(buf.clone()) {
                    let _ = event_tx.send(SerialEvent::Data(s));
                } else {
                    // Send as lossy UTF-8
                    let s = String::from_utf8_lossy(buf).to_string();
                    let _ = event_tx.send(SerialEvent::Data(s));
                }
                buf.clear();
            }
        }
        LineEnding::LF => {
            extract_by_delimiter(buf, b'\n', event_tx);
        }
        LineEnding::CR => {
            extract_by_delimiter(buf, b'\r', event_tx);
        }
        LineEnding::CrLf => {
            extract_by_crlf(buf, event_tx);
        }
    }
}

fn extract_by_delimiter(
    buf: &mut Vec<u8>,
    delim: u8,
    event_tx: &mpsc::SyncSender<SerialEvent>,
) {
    loop {
        if let Some(pos) = buf.iter().position(|&b| b == delim) {
            let line_bytes = buf.drain(..=pos).collect::<Vec<u8>>();
            let line = String::from_utf8_lossy(&line_bytes)
                .trim_end_matches(|c: char| c == '\r' || c == '\n')
                .to_string();
            if !line.is_empty() {
                let _ = event_tx.send(SerialEvent::Data(line));
            }
        } else {
            break;
        }
    }
}

fn extract_by_crlf(buf: &mut Vec<u8>, event_tx: &mpsc::SyncSender<SerialEvent>) {
    loop {
        if let Some(pos) = buf.windows(2).position(|w| w == b"\r\n") {
            let line_bytes: Vec<u8> = buf.drain(..pos + 2).collect();
            let line = String::from_utf8_lossy(&line_bytes)
                .trim_end_matches(|c: char| c == '\r' || c == '\n')
                .to_string();
            if !line.is_empty() {
                let _ = event_tx.send(SerialEvent::Data(line));
            }
        } else {
            break;
        }
    }
}

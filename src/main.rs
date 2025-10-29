// On Arch Linux (EndeavourOS), you must add your user to the "uucp" group with
// "sudo usermod -aG uucp <username>" to access serial ports

// Prevent terminal from running in the background on Windows
#![windows_subsystem = "windows"]

use iced::border::Radius;
use iced::time::{Duration, every};
use iced::widget::{
    button, checkbox, column, combo_box, container, radio, row, scrollable, text, text_input,
};
use iced::{Border, Element, Fill, Size, Subscription, Theme, window};
use serialport::{DataBits, Parity, StopBits};
use std::io::Write;

const VERSION: &str = "v0.7";

fn main() -> iced::Result {
    let rs232_icon = window::icon::from_rgba(include_bytes!("icon.png").to_vec(), 24, 24).ok(); // TESTING

    // Initial Window Settings
    let settings = window::Settings {
        size: Size::new(500.0, 500.0),
        min_size: Some(Size::new(500.0, 500.0)),
        icon: rs232_icon, // TESTING
        ..Default::default()
    };
    // Run App
    iced::application(SerialApp::title, SerialApp::update, SerialApp::view)
        .subscription(SerialApp::subscription)
        .window(settings)
        .theme(SerialApp::theme)
        .run()
}
// App State
struct SerialApp {
    port_list: combo_box::State<String>,
    baud_rate_list: combo_box::State<u32>,
    data_bits_list: combo_box::State<DataBits>,
    parity_list: combo_box::State<Parity>,
    stop_bits_list: combo_box::State<StopBits>,
    selected_port: Option<String>,
    selected_baud_rate: Option<u32>,
    selected_data_bits: Option<DataBits>,
    selected_parity: Option<Parity>,
    selected_stop_bits: Option<StopBits>,
    theme_list: combo_box::State<Theme>,
    selected_theme: Option<Theme>,
    port: Option<Box<dyn serialport::SerialPort>>,
    command: String,
    log_messages: Vec<String>,
    recv_state: RecvState,
    radio_choice: Option<RadioChoice>,
    rx_utf8_checked: bool,
    rx_hex_checked: bool,
    rx_binary_checked: bool,
}
// Default App State
impl Default for SerialApp {
    fn default() -> Self {
        SerialApp::new()
    }
}
// Send Radio
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RadioChoice {
    Utf8,
    Hex,
}
// Listener State
enum RecvState {
    Idle,
    Listening,
}
// App Messages
#[derive(Debug, Clone)]
enum Message {
    ChangeCmd(String),
    SelectPort(String),
    SelectBaudRate(u32),
    SelectDataBits(DataBits),
    SelectParity(Parity),
    SelectStopBits(StopBits),
    SelectTheme(Theme),
    HoverTheme(Theme),
    OpenPort,
    ClosePort,
    Send,
    Recv,
    ToggleListener,
    SelectRadio(RadioChoice),
    CheckBoxUTF8(bool),
    CheckBoxHEX(bool),
    CheckBoxBIN(bool),
}
// App Functions
impl SerialApp {
    // App Title and Version
    fn title(&self) -> String {
        format!("Serial App {VERSION}")
    }
    // Initial App State
    fn new() -> Self {
        let ports = serialport::available_ports()
            .expect("No ports found")
            .iter()
            .map(|port| port.port_name.clone())
            .collect::<Vec<_>>();
        let baud_rates = vec![9600, 19200, 38400, 57600, 115200];
        let data_bits = vec![
            DataBits::Five,
            DataBits::Six,
            DataBits::Seven,
            DataBits::Eight,
        ];
        let parity = vec![Parity::None, Parity::Odd, Parity::Even];
        let stop_bits = vec![StopBits::One, StopBits::Two];
        let themes = Theme::ALL.to_vec();
        Self {
            port_list: combo_box::State::new(ports),
            baud_rate_list: combo_box::State::new(baud_rates),
            data_bits_list: combo_box::State::new(data_bits),
            parity_list: combo_box::State::new(parity),
            stop_bits_list: combo_box::State::new(stop_bits),
            selected_port: None,
            selected_baud_rate: Some(9600),
            selected_data_bits: Some(DataBits::Eight),
            selected_parity: Some(Parity::None),
            selected_stop_bits: Some(StopBits::One),
            theme_list: combo_box::State::new(themes),
            selected_theme: Some(Theme::CatppuccinFrappe),
            port: None,
            command: String::new(),
            log_messages: Vec::new(),
            recv_state: RecvState::Idle,
            radio_choice: Some(RadioChoice::Utf8),
            rx_utf8_checked: false,
            rx_hex_checked: true,
            rx_binary_checked: false,
        }
    }
    // App Logic
    fn update(&mut self, message: Message) {
        match message {
            Message::SelectPort(port) => self.selected_port = Some(port),
            Message::SelectBaudRate(baud_rate) => self.selected_baud_rate = Some(baud_rate),
            Message::SelectDataBits(data_bits) => self.selected_data_bits = Some(data_bits),
            Message::SelectParity(parity) => self.selected_parity = Some(parity),
            Message::SelectStopBits(stop_bits) => self.selected_stop_bits = Some(stop_bits),
            Message::SelectRadio(choice) => self.radio_choice = Some(choice),
            Message::CheckBoxUTF8(clicked) => self.rx_utf8_checked = clicked,
            Message::CheckBoxHEX(clicked) => self.rx_hex_checked = clicked,
            Message::CheckBoxBIN(clicked) => self.rx_binary_checked = clicked,
            Message::ChangeCmd(cmd) => self.command = cmd,
            Message::SelectTheme(theme) => self.selected_theme = Some(theme),
            Message::HoverTheme(theme) => self.selected_theme = Some(theme),
            Message::OpenPort => {
                if self.selected_port.is_none() {
                    self.log_messages.push("No port selected".to_string());
                    return;
                }
                self.port = match serialport::new(
                    self.selected_port.as_deref().unwrap(),
                    self.selected_baud_rate.unwrap(),
                )
                .data_bits(self.selected_data_bits.unwrap())
                .parity(self.selected_parity.unwrap())
                .stop_bits(self.selected_stop_bits.unwrap())
                .timeout(Duration::from_millis(10))
                .open()
                {
                    Ok(port) => {
                        self.log_messages.push(format!(
                            "Successfully opened port '{}'",
                            self.selected_port.as_deref().unwrap()
                        ));
                        Some(port)
                    }
                    Err(e) => {
                        self.log_messages.push(format!(
                            "Failed to open port '{}': {e}",
                            self.selected_port.as_deref().unwrap()
                        ));
                        None
                    }
                }
            }
            Message::ClosePort => {
                if self.port.is_some() {
                    self.port = None;
                    self.log_messages.push("Port closed".to_string());
                    self.recv_state = RecvState::Idle;
                }
            }
            Message::Send => match self.port {
                Some(ref mut port) => {
                    let cmd = &self.command;
                    if self.radio_choice == Some(RadioChoice::Hex) {
                        let hex_string = cmd.replace(" ", "");
                        if !hex_string.len().is_multiple_of(2) {
                            self.log_messages.push("Invalid hex string".to_string());
                            return;
                        }
                        let hex_bytes = match hex::decode(&hex_string) {
                            Ok(decoded_hex) => decoded_hex,
                            Err(e) => {
                                self.log_messages.push(format!("Error decoding hex: {e}"));
                                return;
                            }
                        };
                        match port.write_all(&hex_bytes) {
                            Ok(_) => {}
                            Err(e) => {
                                self.log_messages
                                    .push(format!("Error sending hex command: {e}"));
                                return;
                            }
                        }
                    } else if self.radio_choice == Some(RadioChoice::Utf8) {
                        match port.write_all(cmd.as_bytes()) {
                            Ok(_) => {}
                            Err(e) => {
                                self.log_messages
                                    .push(format!("Error sending utf8 command: {e}"));
                                return;
                            }
                        }
                    }
                    let bytes_sent = cmd.clone().into_bytes().len();
                    self.log_messages
                        .push(format!("Sent {} bytes: {}", bytes_sent, cmd));
                }
                None => {
                    self.log_messages.push("Port not open".to_string());
                }
            },
            Message::Recv => match self.port {
                Some(ref mut port) => {
                    if port.bytes_to_read().unwrap() > 0 {
                        let mut buffer = vec![0; 16];
                        match port.read(&mut buffer) {
                            Ok(b) => {
                                if self.rx_hex_checked {
                                    let hex_string = buffer
                                        .iter()
                                        .map(|byte| format!("{byte:02X}"))
                                        .collect::<Vec<String>>()
                                        .join(" ");
                                    self.log_messages
                                        .push(format!("Received {b} bytes: {hex_string}"));
                                }
                                if self.rx_binary_checked {
                                    let binary_string = buffer
                                        .iter()
                                        .map(|byte| format!("{byte:08b}"))
                                        .collect::<Vec<String>>()
                                        .join(" ");
                                    self.log_messages
                                        .push(format!("Received {b} bytes: {binary_string}"));
                                }
                                if self.rx_utf8_checked {
                                    let utf8_string = String::from_utf8(buffer).unwrap();
                                    self.log_messages
                                        .push(format!("Received {b} bytes: {utf8_string}"));
                                }
                            }
                            Err(e) => {
                                self.log_messages.push(e.to_string());
                            }
                        }
                    }
                }
                None => {
                    self.log_messages.push("Port not open".to_string());
                }
            },
            Message::ToggleListener => {
                if self.port.is_some() {
                    match self.recv_state {
                        RecvState::Idle => {
                            self.recv_state = RecvState::Listening;
                            self.log_messages.push("Listener started".to_string());
                        }
                        RecvState::Listening => {
                            self.recv_state = RecvState::Idle;
                            self.log_messages.push("Listener stopped".to_string());
                        }
                    }
                } else {
                    self.log_messages.push("Port not open".to_string());
                }
            }
        }
    }
    // Listener
    fn subscription(&self) -> Subscription<Message> {
        match self.recv_state {
            RecvState::Idle => Subscription::none(),
            RecvState::Listening => every(Duration::from_millis(10)).map(|_| Message::Recv),
        }
    }
    // App UI
    fn view(&self) -> Element<'_, Message> {
        // Inputs
        let port_list = combo_box(
            &self.port_list,
            "Select a port...",
            self.selected_port.as_ref(),
            Message::SelectPort,
        )
        .padding(10);
        let baud_rate = combo_box(
            &self.baud_rate_list,
            "Baud rate",
            self.selected_baud_rate.as_ref(),
            Message::SelectBaudRate,
        )
        .padding(10);
        let parity = combo_box(
            &self.parity_list,
            "Parity",
            self.selected_parity.as_ref(),
            Message::SelectParity,
        )
        .padding(10);
        let data_bits = combo_box(
            &self.data_bits_list,
            "Data bits",
            self.selected_data_bits.as_ref(),
            Message::SelectDataBits,
        )
        .padding(10);
        let stop_bits = combo_box(
            &self.stop_bits_list,
            "Stop bits",
            self.selected_stop_bits.as_ref(),
            Message::SelectStopBits,
        )
        .padding(10);
        let theme_list = combo_box(
            &self.theme_list,
            "Change theme...",
            self.selected_theme.as_ref(),
            Message::SelectTheme,
        )
        .on_option_hovered(Message::HoverTheme)
        .padding(10)
        .width(200);
        let command = text_input("Enter command...", &self.command)
            .on_input(Message::ChangeCmd)
            .on_submit(Message::Send)
            .padding(10);
        let tx_type = text("Command type:");
        let tx_utf8 = radio(
            "UTF-8",
            RadioChoice::Utf8,
            self.radio_choice,
            Message::SelectRadio,
        );
        let tx_hex = radio(
            "HEX",
            RadioChoice::Hex,
            self.radio_choice,
            Message::SelectRadio,
        );
        let rx_type = text("Receive as:");
        let rx_utf8 = checkbox("UTF-8", self.rx_utf8_checked).on_toggle(Message::CheckBoxUTF8);
        let rx_hex = checkbox("HEX", self.rx_hex_checked).on_toggle(Message::CheckBoxHEX);
        let rx_bin = checkbox("BIN", self.rx_binary_checked).on_toggle(Message::CheckBoxBIN);

        // Buttons
        let port_toggle = if self.port.is_some() {
            button("Close Port")
                .padding(10)
                .style(button::danger)
                .on_press(Message::ClosePort)
        } else {
            button("Open Port").padding(10).on_press(Message::OpenPort)
        };
        let send = button("Send")
            .padding(10)
            .style(button::success)
            .on_press(Message::Send);
        let recv_toggle = {
            match &self.recv_state {
                RecvState::Idle => button("Start Listener")
                    .padding(10)
                    .style(button::success)
                    .on_press(Message::ToggleListener),
                RecvState::Listening => button("Stop Listener")
                    .padding(10)
                    .style(button::danger)
                    .on_press(Message::ToggleListener),
            }
        };
        // Log
        let mut log_column = column![];
        for i in &self.log_messages {
            log_column = log_column.push(i.as_str());
        }
        let log = container(
            scrollable(log_column)
                .anchor_bottom()
                .width(Fill)
                .height(Fill),
        )
        .padding(10)
        .style(|theme: &Theme| container::Style {
            border: Border {
                color: theme.palette().success,
                width: 1.0,
                radius: Radius::new(3.0),
            },
            ..container::Style::default()
        });
        // Layout
        container(
            column![
                row![port_list, port_toggle, recv_toggle].spacing(20),
                row![baud_rate, data_bits, parity, stop_bits].spacing(20),
                row![rx_type, rx_hex, rx_bin, rx_utf8].spacing(20),
                row![log],
                row![tx_type, tx_utf8, tx_hex].spacing(20),
                row![command, send].spacing(20),
                row![theme_list].spacing(20),
            ]
            .spacing(20),
        )
        .padding(20)
        .into()
    }
    // Initial Theme
    fn theme(&self) -> Theme {
        self.selected_theme.as_ref().unwrap().clone()
    }
}

// On Arch Linux (EndeavourOS), you must add your user to the "uucp" group with
// "sudo usermod -aG uucp <username>" to access serial ports

use iced::border::Radius;
use iced::time::{Duration, every};
use iced::widget::{
    button, checkbox, column, combo_box, container, radio, row, scrollable, text, text_input,
};
use iced::{Border, Element, Fill, Size, Subscription, Theme};
use std::io::Write;

const VERSION: &str = "v0.6";

fn main() -> iced::Result {
    // Initial Window Settings
    let settings = iced::window::Settings {
        size: Size::new(500.0, 500.0),
        min_size: Some(Size::new(500.0, 500.0)),
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
    selected_port: Option<String>,
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
    UTF8,
    HEX,
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
        format!("Serial App {}", VERSION)
    }
    // Initial App State
    fn new() -> Self {
        let ports = serialport::available_ports()
            .expect("No ports found")
            .iter()
            .map(|port| port.port_name.clone())
            .collect::<Vec<_>>();
        let mut themes = Theme::ALL.to_vec();
        themes.remove(0); // removed Light theme from selection
        Self {
            port_list: combo_box::State::new(ports),
            selected_port: None,
            theme_list: combo_box::State::new(themes),
            selected_theme: None,
            port: None,
            command: String::new(),
            log_messages: Vec::new(),
            recv_state: RecvState::Idle,
            radio_choice: Some(RadioChoice::UTF8),
            rx_utf8_checked: false,
            rx_hex_checked: true,
            rx_binary_checked: false,
        }
    }
    // App Logic
    fn update(&mut self, message: Message) {
        match message {
            Message::ChangeCmd(cmd) => self.command = cmd,
            Message::SelectPort(port) => self.selected_port = Some(port),
            Message::SelectTheme(theme) => self.selected_theme = Some(theme),
            Message::HoverTheme(theme) => {
                self.selected_theme = Some(theme);
            }
            Message::OpenPort => {
                if self.selected_port.is_none() {
                    self.log_messages.push("No port selected".to_string());
                    return;
                }
                self.port = match serialport::new(self.selected_port.as_deref().unwrap(), 9600)
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
                    if self.radio_choice == Some(RadioChoice::HEX) {
                        let hex_string = cmd.replace(" ", "");
                        if hex_string.len() % 2 != 0 {
                            self.log_messages.push("Invalid hex string".to_string());
                            return;
                        }
                        let hex_bytes =
                            match hex::decode(&hex_string) {
                                Ok(decoded_hex) => {decoded_hex}
                                Err(e) => {
                                    self.log_messages.push(format!("Error decoding hex: {}", e));
                                    return;
                                }
                            };
                        match port.write_all(&hex_bytes) {
                            Ok(_) => {}
                            Err(e) => {
                                self.log_messages.push(format!("Error sending hex command: {}", e));
                                return;
                            }
                        }
                            
                    } else if self.radio_choice == Some(RadioChoice::UTF8) {
                        match port.write_all(cmd.as_bytes()) {
                            Ok(_) => {}
                            Err(e) => {
                                self.log_messages.push(format!("Error sending utf8 command: {}", e));
                                return;
                            }
                        }
                    }
                    self.log_messages.push("Sent: ".to_string() + cmd);
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
                            Ok(_) => {
                                if self.rx_hex_checked {
                                    let hex_string = buffer
                                        .iter()
                                        .map(|byte| format!("{:02X}", byte))
                                        .collect::<Vec<String>>()
                                        .join(" ");
                                    self.log_messages.push(format!("Received: {}", hex_string));
                                }
                                if self.rx_binary_checked {
                                    let binary_string = buffer
                                        .iter()
                                        .map(|byte| format!("{:08b}", byte))
                                        .collect::<Vec<String>>()
                                        .join(" ");
                                    self.log_messages
                                        .push(format!("Received: {}", binary_string));
                                }
                                if self.rx_utf8_checked {
                                    let utf8_string = String::from_utf8(buffer).unwrap();
                                    self.log_messages.push(format!("Received: {}", utf8_string));
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
                        RecvState::Listening { .. } => {
                            self.recv_state = RecvState::Idle;
                            self.log_messages.push("Listener stopped".to_string());
                        }
                    }
                } else {
                    self.log_messages.push("Port not open".to_string());
                }
            }
            Message::SelectRadio(choice) => {
                self.radio_choice = Some(choice);
            }
            Message::CheckBoxUTF8(clicked) => {
                self.rx_utf8_checked = clicked;
            }
            Message::CheckBoxHEX(clicked) => {
                self.rx_hex_checked = clicked;
            }
            Message::CheckBoxBIN(clicked) => {
                self.rx_binary_checked = clicked;
            }
        }
    }
    // Listener
    fn subscription(&self) -> Subscription<Message> {
        match self.recv_state {
            RecvState::Idle => Subscription::none(),
            RecvState::Listening { .. } => every(Duration::from_millis(10)).map(|_| Message::Recv),
        }
    }
    // App UI
    fn view(&self) -> Element<Message> {
        // Inputs
        let port_list = combo_box(
            &self.port_list,
            "Select a port...",
            self.selected_port.as_ref(),
            Message::SelectPort,
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
            RadioChoice::UTF8,
            self.radio_choice,
            Message::SelectRadio,
        );
        let tx_hex = radio(
            "HEX",
            RadioChoice::HEX,
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
                RecvState::Listening { .. } => button("Stop Listener")
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
        self.selected_theme.as_ref().unwrap_or(&Theme::Dark).clone()
    }
}

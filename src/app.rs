use crate::event::{AppEvent, Event, EventHandler};
use crate::infer::InferenceEngine;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

#[derive(Debug, Clone, PartialEq)]
pub enum SafetyStatus {
    Unknown,
    Safe,
    Unsafe,
}

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub focused_pane: usize,
    pub input_text: String,
    pub response_text: String,
    pub safety_check_text: String,
    pub input_cursor: usize,
    pub response_cursor: usize,
    pub events: EventHandler,
    pub client: InferenceEngine,
    pub is_loading_completion: bool,
    pub is_loading_safety_check: bool,
    pub safety_status: SafetyStatus,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            focused_pane: 0,
            input_text: String::new(),
            response_text: String::new(),
            safety_check_text: String::new(),
            input_cursor: 0,
            response_cursor: 0,
            events: EventHandler::new(),
            // TODO: Handle gracefully
            client: InferenceEngine::new()
                .unwrap_or_else(|e| panic!("Failed to create client: {}", e)),
            is_loading_completion: false,
            is_loading_safety_check: false,
            safety_status: SafetyStatus::Unknown,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event) => self.handle_key_events(key_event)?,
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                    AppEvent::RequestCompletion(input) => {
                        self.handle_completion_request(input).await?;
                    }
                    AppEvent::CompletionResponse(response) => {
                        self.is_loading_completion = false;
                        self.response_text = response.clone();
                        self.response_cursor = self.response_text.len();
                        self.check_completion_request(response).await;
                    }
                    AppEvent::CompletionError(error) => {
                        self.is_loading_completion = false;
                        self.response_text = format!("Error: {}", error);
                        self.response_cursor = self.response_text.len();
                    }
                    AppEvent::SafetyCheckResponse(response) => {
                        self.is_loading_safety_check = false;
                        self.safety_check_text = response.clone();

                        if response.trim().starts_with('Y') {
                            self.safety_status = SafetyStatus::Safe;
                        } else if response.trim().starts_with('N') {
                            self.safety_status = SafetyStatus::Unsafe;
                        } else {
                            self.safety_status = SafetyStatus::Unknown;
                        }
                    }
                    AppEvent::SafetyCheckError(error) => {
                        self.is_loading_safety_check = false;
                        self.safety_check_text = format!("Safety check error: {}", error);
                        self.safety_status = SafetyStatus::Unknown;
                    }
                    AppEvent::ExecuteCommand(command) => {
                        self.execute_command(command)?;
                    }
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        // TODO: Messy, but okay for now
        match key_event.code {
            KeyCode::Esc => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Up | KeyCode::Down => {
                self.focused_pane = if self.focused_pane == 0 { 1 } else { 0 };
            }
            KeyCode::Enter if self.focused_pane == 0 => {
                let input = self.input_text.clone();
                self.events.send(AppEvent::RequestCompletion(input));
            }
            KeyCode::Enter if self.focused_pane == 1 => {
                let command = self.response_text.clone();
                self.events.send(AppEvent::ExecuteCommand(command));
            }
            KeyCode::Char(c) if self.focused_pane == 0 => {
                // Handle text input when top pane is focused
                self.input_text.insert(self.input_cursor, c);
                self.input_cursor += 1;
            }
            KeyCode::Backspace if self.focused_pane == 0 => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.input_text.remove(self.input_cursor);
                }
            }
            KeyCode::Char(c) if self.focused_pane == 1 => {
                // Handle text input when middle pane is focused
                self.response_text.insert(self.response_cursor, c);
                self.response_cursor += 1;
            }
            KeyCode::Backspace if self.focused_pane == 1 => {
                if self.response_cursor > 0 {
                    self.response_cursor -= 1;
                    self.response_text.remove(self.response_cursor);
                }
            }
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Handle completion request asynchronously.
    async fn handle_completion_request(&mut self, input: String) -> color_eyre::Result<()> {
        if input.trim().is_empty() {
            return Ok(());
        }

        self.is_loading_completion = true;
        self.safety_status = SafetyStatus::Unknown;

        let client = self.client.clone();
        let sender = self.events.sender.clone();
        tokio::spawn(async move {
            match client.imagine_command(input).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        let _ = sender.send(Event::App(AppEvent::CompletionResponse(
                            choice.message.content.clone(),
                        )));
                    } else {
                        let _ = sender.send(Event::App(AppEvent::CompletionError(
                            "No response received".to_string(),
                        )));
                    }
                }
                Err(e) => {
                    let _ = sender.send(Event::App(AppEvent::CompletionError(e.to_string())));
                }
            }
        });

        Ok(())
    }

    async fn check_completion_request(&mut self, input: String) {
        self.is_loading_safety_check = true;

        let infer = self.client.clone();
        let sender = self.events.sender.clone();

        tokio::spawn(async move {
            match infer.inspect_command(input).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        let _ = sender.send(Event::App(AppEvent::SafetyCheckResponse(
                            choice.message.content.clone(),
                        )));
                    } else {
                        let _ = sender.send(Event::App(AppEvent::SafetyCheckError(
                            "No safety check response received".to_string(),
                        )));
                    }
                }
                Err(e) => {
                    let _ = sender.send(Event::App(AppEvent::SafetyCheckError(e.to_string())));
                }
            }
        });
    }

    /// Execute the command and replace the current process.
    fn execute_command(&mut self, command: String) -> color_eyre::Result<()> {
        // Sorry windows users.
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        if command.trim().is_empty() {
            return Ok(());
        }

        ratatui::restore();

        // IDEA: Also place the command on the pasteboard?

        // I originally had wanted to 'stage' the command, as a final sanity check.
        // Escape codes won't modify the typeahead. Fish does have a commandline function,
        // but it has to be called inside a fish shell, I can't spawn a subshell and run
        // that inside.
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        let err = cmd.exec();

        Err(color_eyre::eyre::eyre!(
            "Failed to execute command: {}",
            err
        ))
    }
}

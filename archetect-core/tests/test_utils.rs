use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use camino::Utf8PathBuf;
use rhai::{Dynamic, Map};

use archetect_api::{
    sync_io_channel, ClientIoHandle, ClientMessage, ScriptMessage, SyncClientIoHandle,
    BoolPromptInfo, EditorPromptInfo, IntPromptInfo, ListPromptInfo,
    MultiSelectPromptInfo, SelectPromptInfo, TextPromptInfo,
    WriteDirectoryInfo, WriteFileInfo,
};
use archetect_core::archetype::render_context::RenderContext;
use archetect_core::configuration::Configuration;
use archetect_core::errors::ArchetectError;
use archetect_core::Archetect;

pub fn get_archetype_path(rs_file: &str) -> Utf8PathBuf {
    let rust_file = Utf8PathBuf::from(rs_file);
    let parent = rust_file.parent().expect("Directory");
    let mut test_directory = Utf8PathBuf::new();
    for component in parent.components().skip(1) {
        test_directory = test_directory.join(component);
    }
    test_directory.join(rust_file.file_stem().expect("Archetype Directory"))
}

pub struct TestHarness {
    handle: SyncClientIoHandle,
    status_rx: Receiver<bool>,
}

impl TestHarness {
    pub fn new(
        test_file: &str,
        configuration: Configuration,
        render_context: RenderContext,

    ) -> Result<TestHarness, ArchetectError> {
        let archetype_dir = get_archetype_path(test_file);

        let (script_handle, client_handle) = sync_io_channel();
        let archetect = Archetect::builder()
            .with_driver(script_handle)
            .with_configuration(configuration)
            .with_temp_layout()?
            .build()?;

        let archetype = archetect.new_archetype(archetype_dir.as_str())?;
        let (status_tx, status_rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || match archetype.render(render_context) {
            Ok(_) => {
                status_tx.send(true).expect("Send Error");
            }
            Err(_err) => {
                status_tx.send(false).expect("Send Error");
            }
        });

        Ok(TestHarness { handle: client_handle, status_rx })
    }

    pub fn respond(&self, response: ClientMessage) {
        self.handle.send(response).expect("Failed to send response");
    }

    pub fn receive(&self) -> ScriptMessage {
        self.handle.receive().expect("Expected ScriptMessage")
    }

    pub fn render_succeeded(&self) -> bool {
        self.status_rx.recv_timeout(Duration::from_millis(100)).expect("Expected Render Status")
    }

    // --- Typed prompt expectations ---

    pub fn expect_text_prompt(&self) -> TextPromptInfo {
        match self.receive() {
            ScriptMessage::PromptForText(info) => info,
            other => panic!("Expected PromptForText, got {:?}", other),
        }
    }

    pub fn expect_int_prompt(&self) -> IntPromptInfo {
        match self.receive() {
            ScriptMessage::PromptForInt(info) => info,
            other => panic!("Expected PromptForInt, got {:?}", other),
        }
    }

    pub fn expect_bool_prompt(&self) -> BoolPromptInfo {
        match self.receive() {
            ScriptMessage::PromptForBool(info) => info,
            other => panic!("Expected PromptForBool, got {:?}", other),
        }
    }

    pub fn expect_select_prompt(&self) -> SelectPromptInfo {
        match self.receive() {
            ScriptMessage::PromptForSelect(info) => info,
            other => panic!("Expected PromptForSelect, got {:?}", other),
        }
    }

    pub fn expect_multiselect_prompt(&self) -> MultiSelectPromptInfo {
        match self.receive() {
            ScriptMessage::PromptForMultiSelect(info) => info,
            other => panic!("Expected PromptForMultiSelect, got {:?}", other),
        }
    }

    pub fn expect_list_prompt(&self) -> ListPromptInfo {
        match self.receive() {
            ScriptMessage::PromptForList(info) => info,
            other => panic!("Expected PromptForList, got {:?}", other),
        }
    }

    pub fn expect_editor_prompt(&self) -> EditorPromptInfo {
        match self.receive() {
            ScriptMessage::PromptForEditor(info) => info,
            other => panic!("Expected PromptForEditor, got {:?}", other),
        }
    }

    // --- Log/output expectations ---

    pub fn expect_log_trace(&self) -> String {
        match self.receive() {
            ScriptMessage::LogTrace(msg) => msg,
            other => panic!("Expected LogTrace, got {:?}", other),
        }
    }

    pub fn expect_log_debug(&self) -> String {
        match self.receive() {
            ScriptMessage::LogDebug(msg) => msg,
            other => panic!("Expected LogDebug, got {:?}", other),
        }
    }

    pub fn expect_log_info(&self) -> String {
        match self.receive() {
            ScriptMessage::LogInfo(msg) => msg,
            other => panic!("Expected LogInfo, got {:?}", other),
        }
    }

    pub fn expect_log_warn(&self) -> String {
        match self.receive() {
            ScriptMessage::LogWarn(msg) => msg,
            other => panic!("Expected LogWarn, got {:?}", other),
        }
    }

    pub fn expect_log_error(&self) -> String {
        match self.receive() {
            ScriptMessage::LogError(msg) => msg,
            other => panic!("Expected LogError, got {:?}", other),
        }
    }

    pub fn expect_display(&self) -> String {
        match self.receive() {
            ScriptMessage::Display(msg) => msg,
            other => panic!("Expected Display, got {:?}", other),
        }
    }

    pub fn expect_print(&self) -> String {
        match self.receive() {
            ScriptMessage::Print(msg) => msg,
            other => panic!("Expected Print, got {:?}", other),
        }
    }

    // --- Write expectations (auto-Ack) ---

    pub fn expect_write_directory(&self) -> String {
        match self.receive() {
            ScriptMessage::WriteDirectory(info) => {
                self.respond(ClientMessage::Ack);
                info.path
            }
            other => panic!("Expected WriteDirectory, got {:?}", other),
        }
    }

    pub fn expect_write_file(&self) -> WriteFileInfo {
        match self.receive() {
            ScriptMessage::WriteFile(info) => {
                self.respond(ClientMessage::Ack);
                info
            }
            other => panic!("Expected WriteFile, got {:?}", other),
        }
    }

    // --- Convenience response methods ---

    pub fn respond_text(&self, value: &str) {
        self.respond(ClientMessage::String(value.to_string()));
    }

    pub fn respond_int(&self, value: i64) {
        self.respond(ClientMessage::Integer(value));
    }

    pub fn respond_bool(&self, value: bool) {
        self.respond(ClientMessage::Boolean(value));
    }

    pub fn respond_array(&self, values: Vec<&str>) {
        self.respond(ClientMessage::Array(
            values.into_iter().map(String::from).collect(),
        ));
    }

    pub fn respond_none(&self) {
        self.respond(ClientMessage::None);
    }
}

// --- Builder for reducing test boilerplate ---

pub struct TestHarnessBuilder {
    test_file: String,
    configuration: Configuration,
    answers: Map,
    switches: Vec<String>,
    use_defaults_all: bool,
    destination: Utf8PathBuf,
}

impl TestHarnessBuilder {
    pub fn new(test_file: &str) -> Self {
        TestHarnessBuilder {
            test_file: test_file.to_string(),
            configuration: Configuration::default(),
            answers: Map::new(),
            switches: Vec::new(),
            use_defaults_all: false,
            destination: Utf8PathBuf::new(),
        }
    }

    pub fn headless(mut self) -> Self {
        self.configuration = self.configuration.with_headless(true);
        self
    }

    pub fn with_switch(mut self, switch: &str) -> Self {
        self.switches.push(switch.to_string());
        self
    }

    pub fn with_answer(mut self, key: &str, value: impl Into<Dynamic>) -> Self {
        self.answers.insert(key.into(), value.into());
        self
    }

    pub fn with_destination(mut self, destination: Utf8PathBuf) -> Self {
        self.destination = destination;
        self
    }

    pub fn use_defaults_all(mut self) -> Self {
        self.use_defaults_all = true;
        self
    }

    pub fn build(self) -> Result<TestHarness, ArchetectError> {
        let mut render_context = RenderContext::new(self.destination, self.answers);
        for switch in self.switches {
            render_context = render_context.with_switch(switch);
        }
        if self.use_defaults_all {
            render_context = render_context.with_use_defaults_all(true);
        }
        TestHarness::new(&self.test_file, self.configuration, render_context)
    }
}

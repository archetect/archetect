use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use camino::Utf8PathBuf;

use archetect_api::{SyncClientIoHandle, ScriptMessage, ClientMessage, SyncIoDriver, ClientIoHandle};
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
    client_handle: SyncClientIoHandle,
    status_rx: Receiver<bool>,
}

impl TestHarness {
    pub fn execute(
        test_file: &str,
        configuration: Configuration,
        render_context: RenderContext,

    ) -> Result<TestHarness, ArchetectError> {
        let archetype_dir = get_archetype_path(test_file);

        let (script_handle, client_handle) = SyncIoDriver::new().split();

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

        Ok(TestHarness { client_handle, status_rx })
    }

    pub fn respond(&self, response: ClientMessage) {
        self.client_handle.send(response);
    }

    pub fn receive(&self) -> ScriptMessage {
        self.client_handle.receive().expect("Expected Message")
    }

    pub fn render_succeeded(&self) -> bool {
        self.status_rx.recv_timeout(Duration::from_millis(100)).expect("Expected Render Status")
    }
}

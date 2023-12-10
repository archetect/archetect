use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use camino::Utf8PathBuf;

use archetect_api::{api_driver_and_handle, ApiIoHandle, CommandRequest, CommandResponse};
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
    handle: ApiIoHandle,
    status_rx: Receiver<bool>,
}

impl TestHarness {
    pub fn new(
        test_file: &str,
        configuration: Configuration,
        render_context: RenderContext,

    ) -> Result<TestHarness, ArchetectError> {
        let archetype_dir = get_archetype_path(test_file);

        let (driver, handle) = api_driver_and_handle();
        let archetect = Archetect::builder()
            .with_driver(driver)
            .with_configuration(configuration)
            .with_temp_layout()?
            .build()?;

        let archetype = archetect.new_archetype(archetype_dir.as_str(), false)?;
        let (status_tx, status_rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || match archetype.render(render_context) {
            Ok(_) => {
                status_tx.send(true).expect("Send Error");
            }
            Err(_err) => {
                status_tx.send(false).expect("Send Error");
            }
        });

        Ok(TestHarness { handle, status_rx })
    }

    pub fn respond(&self, response: CommandResponse) {
        self.handle.respond(response);
    }

    pub fn receive(&self) -> CommandRequest {
        self.handle.receive()
    }

    pub fn render_succeeded(&self) -> bool {
        self.status_rx.recv_timeout(Duration::from_millis(100)).expect("Expected Render Status")
    }
}

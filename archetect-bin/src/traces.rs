use std::{fmt, io};

use ansi_term::Colour;
use clap::ArgMatches;
use tracing::{Event, Level, Subscriber};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::registry::LookupSpan;

pub fn initialize(args: &ArgMatches) {
    if let Some(_args) = args.subcommand_matches("server") {
        server_tracing();
    } else {
        client_tracing(args.get_count("verbosity"));
    }
}

pub fn server_tracing() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        .with_target(false)
        // Build the subscriber
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Proper Tracing Configuration");
}

pub fn client_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    let subscriber = tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_max_level(level)
        .event_format(ClientFormatter)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Proper Tracing Configuration");
}

pub struct ClientFormatter;

impl<S, N> FormatEvent<S, N> for ClientFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &Event<'_>) -> std::fmt::Result {
        let meta = event.metadata();
        let fmt_level = FmtLevel::new(meta.level(), writer.has_ansi_escapes());
        write!(writer, "{}: ", fmt_level)?;
        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

struct FmtLevel<'a> {
    level: &'a Level,
    ansi: bool,
}

impl<'a> FmtLevel<'a> {
    pub(crate) fn new(level: &'a Level, ansi: bool) -> Self {
        Self { level, ansi }
    }
}

const ARCHETECT_STR: &str = "archetect";

impl<'a> fmt::Display for FmtLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ansi {
            match *self.level {
                Level::TRACE => write!(f, "{}", Colour::Fixed(245 /* gray */).paint(ARCHETECT_STR)),
                Level::DEBUG => write!(f, "{}", Colour::Fixed(24 /* blue */).paint(ARCHETECT_STR)),
                Level::INFO => write!(f, "{}", Colour::Fixed(34 /* green */).paint(ARCHETECT_STR)),
                Level::WARN => write!(f, "{}", Colour::Fixed(208 /* orange */).paint(ARCHETECT_STR)),
                Level::ERROR => write!(f, "{}", Colour::Fixed(9 /* red */).paint(ARCHETECT_STR)),
            }
        } else {
            match *self.level {
                Level::TRACE => f.pad(ARCHETECT_STR),
                Level::DEBUG => f.pad(ARCHETECT_STR),
                Level::INFO => f.pad(ARCHETECT_STR),
                Level::WARN => f.pad(ARCHETECT_STR),
                Level::ERROR => f.pad(ARCHETECT_STR),
            }
        }
    }
}

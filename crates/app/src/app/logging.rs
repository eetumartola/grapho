use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

const MAX_LOG_LINES: usize = 500;

#[derive(Clone)]
pub(crate) struct ConsoleBuffer {
    lines: Arc<Mutex<VecDeque<String>>>,
}

impl ConsoleBuffer {
    pub(crate) fn new() -> Self {
        Self {
            lines: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub(crate) fn push_line(&self, line: String) {
        let mut lines = self.lines.lock().expect("console buffer lock");
        lines.push_back(line);
        while lines.len() > MAX_LOG_LINES {
            lines.pop_front();
        }
    }

    pub(crate) fn snapshot(&self) -> Vec<String> {
        let lines = self.lines.lock().expect("console buffer lock");
        lines.iter().cloned().collect()
    }
}

struct ConsoleMakeWriter {
    buffer: ConsoleBuffer,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ConsoleMakeWriter {
    type Writer = ConsoleWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ConsoleWriter {
            buffer: self.buffer.clone(),
        }
    }
}

struct ConsoleWriter {
    buffer: ConsoleBuffer,
}

impl io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        for line in text.lines() {
            self.buffer.push_line(line.to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        let _ = io::stdout().write_all(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = io::stdout().flush();
        Ok(())
    }
}

pub(crate) fn setup_tracing() -> (ConsoleBuffer, Arc<AtomicU8>) {
    let console = ConsoleBuffer::new();
    let log_level_state = Arc::new(AtomicU8::new(level_filter_to_u8(LevelFilter::INFO)));
    let filter_state = log_level_state.clone();
    let filter_layer = tracing_subscriber::filter::filter_fn(move |metadata| {
        let level = match filter_state.load(Ordering::Relaxed) {
            value if value == level_filter_to_u8(LevelFilter::ERROR) => Level::ERROR,
            value if value == level_filter_to_u8(LevelFilter::WARN) => Level::WARN,
            value if value == level_filter_to_u8(LevelFilter::INFO) => Level::INFO,
            value if value == level_filter_to_u8(LevelFilter::DEBUG) => Level::DEBUG,
            _ => Level::TRACE,
        };
        metadata.level() <= &level
    });
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(ConsoleMakeWriter {
            buffer: console.clone(),
        });
    #[cfg(target_arch = "wasm32")]
    let fmt_layer = fmt_layer.without_time();

    tracing_subscriber::registry()
        .with(fmt_layer.with_filter(filter_layer))
        .init();

    (console, log_level_state)
}

pub(crate) fn level_filter_to_u8(level: LevelFilter) -> u8 {
    match level {
        LevelFilter::OFF => 0,
        LevelFilter::ERROR => 1,
        LevelFilter::WARN => 2,
        LevelFilter::INFO => 3,
        LevelFilter::DEBUG => 4,
        LevelFilter::TRACE => 5,
    }
}

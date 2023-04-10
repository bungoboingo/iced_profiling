//! A simple profiler for Iced.

use std::fmt::{Debug, Formatter};
use tracing_subscriber::prelude::*;
use tracing_subscriber::Registry;

#[cfg(feature = "chrome")]
use {
    std::ffi::OsStr,
    std::path::Path,
    std::time::Duration,
    tracing_subscriber::fmt::{format::DefaultFields, FormattedFields},
};

pub struct Profiler {
    #[cfg(feature = "chrome")]
    /// [`FlushGuard`] must be kept alive for accurate tracing with chrome.
    _guard: tracing_chrome::FlushGuard,
}

impl Debug for Profiler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Profiler ---- Chrome? {:?}, Tracy {:?}",
            cfg!(feature = "chrome"),
            cfg!(feature = "tracy")
        )
    }
}

impl Profiler {
    /// Initializes the [`Profiler`].
    pub fn init() -> Self {
        // Registry stores the spans & generates unique span IDs
        let subscriber = Registry::default();

        #[cfg(feature = "chrome")]
        let (chrome_layer, guard) = {
            let mut layer = tracing_chrome::ChromeLayerBuilder::new();

            let default_path = Path::new(env!("CARGO_MANIFEST_DIR"));
            let curr_exe = std::env::current_exe().unwrap_or_else(|_| default_path.to_path_buf());
            let out_dir = curr_exe.parent().unwrap_or(default_path).join("traces");

            // Optional configurable env var: CHROME_TRACE_FILE=/path/to/trace_file/file.json,
            // for uploading to chrome://tracing (old) or ui.perfetto.dev (new).
            if let Ok(path) = std::env::var("CHROME_TRACE_FILE") {
                layer = layer.file(path);
            } else if std::fs::create_dir_all(&out_dir).is_ok() {
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(Duration::from_millis(0))
                    .as_millis();

                let curr_exe_name = curr_exe
                    .file_name()
                    .unwrap_or_else(|| OsStr::new("trace"))
                    .to_str()
                    .unwrap_or("trace");

                let path = out_dir.join(format!("{curr_exe_name}_trace_{time}.json"));

                layer = layer.file(path);
            } else {
                layer = layer.file(env!("CARGO_MANIFEST_DIR"))
            }

            let (chrome_layer, guard) = layer
                .name_fn(Box::new(|event_or_span| match event_or_span {
                    tracing_chrome::EventOrSpan::Event(event) => event.metadata().name().into(),
                    tracing_chrome::EventOrSpan::Span(span) => {
                        if let Some(fields) = span
                            .extensions()
                            .get::<FormattedFields<DefaultFields>>() {
                            format!("{}: {}", span.metadata().name(), fields.fields.as_str())
                        } else {
                            span.metadata().name().into()
                        }
                    }
                }))
                .build();

            (chrome_layer, guard)
        };

        #[cfg(feature = "tracy")]
        let tracy_layer = tracing_tracy::TracyLayer::new();

        let fmt_layer = tracing_subscriber::fmt::Layer::default();

        //TODO might need a tracy layer, we shall see

        let subscriber = subscriber.with(fmt_layer);

        #[cfg(feature = "chrome")]
        let subscriber = subscriber.with(chrome_layer);
        #[cfg(feature = "tracy")]
        let subscriber = subscriber.with(tracy_layer);

        // create dispatcher which will forward span events to the subscriber
        // this can only be set once or will panic
        tracing::subscriber::set_global_default(subscriber)
            .expect("Tracer could not set the global default subscriber.");

        Profiler {
            #[cfg(feature = "chrome")]
            _guard: guard,
        }
    }
}

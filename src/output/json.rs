use super::{Event, OutputSink};
use std::io::{self, Write};

pub struct JsonSink<W: Write + Send> {
    writer: W,
}

impl<W: Write + Send> JsonSink<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

pub fn stdout() -> JsonSink<io::Stdout> {
    JsonSink::new(io::stdout())
}

impl<W: Write + Send> OutputSink for JsonSink<W> {
    fn emit(&mut self, event: &Event) {
        let line = serde_json::to_string(event).expect("event serialization");
        let _ = writeln!(self.writer, "{line}");
        let _ = self.writer.flush();
    }
    fn finish(&mut self) {
        let _ = self.writer.flush();
    }
}

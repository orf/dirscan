use std::fs::File;
use std::io;

use crate::directory_stat::DirectoryStat;
use std::io::Write;
use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(EnumString, EnumVariantNames, Display)]
#[strum(serialize_all = "kebab_case")]
pub enum Format {
    Json,
    Csv,
}

impl Format {
    pub fn parse_file(&self, file: File) -> Box<dyn Iterator<Item = DirectoryStat>> {
        let reader = io::BufReader::new(file);
        match self {
            Self::Json => Box::new(
                serde_json::Deserializer::from_reader(reader)
                    .into_iter::<DirectoryStat>()
                    .map(|f| f.unwrap()),
            ),
            Self::Csv => Box::new(
                csv::Reader::from_reader(reader)
                    .into_deserialize::<DirectoryStat>()
                    .map(|f| f.unwrap()),
            ),
        }
    }

    pub fn get_writer(&self, file: Box<dyn io::Write>) -> Box<dyn FormatWriter> {
        match self {
            Self::Json => Box::new(JsonWriter::new(file)),
            Self::Csv => Box::new(CSVWriter::new(file)),
        }
    }
}

pub trait FormatWriter {
    fn new(_: Box<dyn io::Write>) -> Self
    where
        Self: Sized;
    fn write_stat(&mut self, stat: &DirectoryStat) -> io::Result<()>;
}

pub struct JsonWriter {
    writer: Box<dyn io::Write>,
}

impl FormatWriter for JsonWriter {
    fn new(writer: Box<dyn io::Write>) -> Self {
        JsonWriter { writer }
    }

    fn write_stat(&mut self, stat: &DirectoryStat) -> io::Result<()> {
        let res = serde_json::to_vec(stat).expect("Error serializing directory stat");
        self.writer.write_all(&res)?;
        writeln!(self.writer)?;
        Ok(())
    }
}

pub struct CSVWriter {
    csv_writer: csv::Writer<Box<dyn io::Write>>,
}

impl FormatWriter for CSVWriter {
    fn new(writer: Box<dyn Write>) -> Self {
        let csv_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_writer(writer);
        CSVWriter { csv_writer }
    }

    fn write_stat(&mut self, stat: &DirectoryStat) -> io::Result<()> {
        self.csv_writer.serialize(stat)?;
        Ok(())
    }
}

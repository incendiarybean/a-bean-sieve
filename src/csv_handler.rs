use std::fs;

use image::EncodableLayout;
use serde::{de::DeserializeOwned, Serialize};

pub fn read_from_csv<CSVRecord>(file_path: String) -> Result<Vec<CSVRecord>, csv::Error>
where
    CSVRecord: DeserializeOwned,
{
    let file = fs::read(&file_path)?;
    let mut rows: Vec<CSVRecord> = Vec::new();
    let mut raw_csv = csv::Reader::from_reader(file.as_bytes());

    for result in raw_csv.deserialize() {
        let row: CSVRecord = result?;
        rows.push(row);
    }

    Ok(rows)
}

pub fn write_csv_from_vec<CSVRecord>(
    file_path: String,
    headers: Vec<String>,
    records: Vec<CSVRecord>,
) -> Result<(), csv::Error>
where
    CSVRecord: Serialize,
{
    fs::write(&file_path, "")?;
    let mut wtr = csv::Writer::from_path(&file_path)?;
    wtr.serialize(headers)?;

    for item in records {
        wtr.serialize(item)?;
    }

    wtr.flush()?;
    Ok(())
}

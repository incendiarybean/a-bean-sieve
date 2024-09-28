use std::fs::File;

use serde::{de::DeserializeOwned, Serialize};

pub fn read_from_csv<CSVRecord, P: AsRef<std::path::Path>>(
    file_path: P,
) -> Result<Vec<CSVRecord>, csv::Error>
where
    CSVRecord: DeserializeOwned,
{
    let file = File::open(file_path)?;
    let reader = std::io::BufReader::new(file);

    let mut records: Vec<CSVRecord> = Vec::new();
    let mut csv = csv::Reader::from_reader(reader);

    for result in csv.deserialize() {
        let row: CSVRecord = result?;
        records.push(row);
    }

    Ok(records)
}

pub fn write_csv_from_vec<CSVRecord, P: AsRef<std::path::Path>>(
    file_path: P,
    headers: Vec<&str>,
    records: Vec<CSVRecord>,
) -> Result<(), csv::Error>
where
    CSVRecord: Serialize,
    P: Clone,
{
    File::create(file_path.clone())?;
    let mut writer = csv::Writer::from_path(file_path)?;

    writer.serialize(headers)?;

    for record in records {
        writer.serialize(record)?;
    }

    writer.flush()?;

    Ok(())
}

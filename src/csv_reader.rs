use serde::de::DeserializeOwned;

pub fn read_from_csv<CSVRecord>(file_path: &str) -> Result<Vec<CSVRecord>, std::io::Error>
where
    CSVRecord: DeserializeOwned,
{
    let mut rows: Vec<CSVRecord> = Vec::new();
    let mut raw_csv = csv::Reader::from_path(file_path)?;

    for result in raw_csv.deserialize() {
        let row: CSVRecord = result?;
        rows.push(row);
    }

    Ok(rows)
}

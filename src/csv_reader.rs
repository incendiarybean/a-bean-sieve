use serde::de::DeserializeOwned;

pub fn read_from_csv<CSVRecord>(file: &[u8]) -> Result<Vec<CSVRecord>, csv::Error>
where
    CSVRecord: DeserializeOwned,
{
    let mut rows: Vec<CSVRecord> = Vec::new();
    let mut raw_csv = csv::Reader::from_reader(file);

    for result in raw_csv.deserialize() {
        let row: CSVRecord = result?;
        rows.push(row);
    }

    Ok(rows)
}

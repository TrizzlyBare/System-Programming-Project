use csv::Writer;
use regex::Regex;
use reqwest;
use scraper::{Html, Selector};
use select::document::Document;
use select::predicate::Name;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::io::Write;
use xml::writer::{EmitterConfig, XmlEvent};

const OUTPUT_HTML_PATH: &str = "output.html";
const OUTPUT_JSON_PATH: &str = "output.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let link = get_user_input_link()?;
    let html = get_html_data(&link).await?;

    let file_type = file_type_selection()?;
    match file_type.as_str() {
        "1" => {
            let json = html_to_json(&html)?;
            save_data_to_file(&serde_json::to_string_pretty(&json)?, OUTPUT_JSON_PATH, "JSON")?;
        }
        "2" => {
            html_to_csv(&html)?;
        }
        "3" => {
            html_to_xml(&html)?;
        }
        "4" => {
            html_to_toml(&html)?;
        }
        "5" => {
            html_to_yaml(&html)?;
        }
        "6" => {
            html_to_msgpack(&html)?;
        }
        _ => {
            println!("Invalid file type");
        }
    }
    Ok(())
}

fn get_user_input_link() -> Result<String, io::Error> {
    loop {
        print!("Enter the link to the website you want to scrape: ");
        io::stdout().flush()?;
        let mut link = String::new();
        io::stdin().read_line(&mut link)?;

        let trimmed_link = link.trim();
        if trimmed_link.is_empty() {
            println!("Please provide a valid link.");
        } else {
            return Ok(trimmed_link.to_string());
        }
    }
}

fn file_type_selection() -> Result<String, io::Error> {
    loop {
        print!("Enter the file type you want to save the data to (JSON (1) / CSV (2) / XML (3) / TOML (4) / YAML (5) / MessagePack (6)): ");
        io::stdout().flush()?;
        let mut file_type = String::new();
        io::stdin().read_line(&mut file_type)?;

        let trimmed_file_type = file_type.trim();
        if !(1..=6).any(|x| trimmed_file_type == x.to_string()) {
            println!("Please enter a valid file type (1-6).");
        } else {
            return Ok(trimmed_file_type.to_string());
        }
    }
}

async fn get_html_data(link: &str) -> Result<String, Box<dyn Error>> {
    let file_exists = fs::metadata(OUTPUT_HTML_PATH).is_ok();

    if file_exists {
        fs::remove_file(OUTPUT_HTML_PATH)?;
    }

    let res = request(link).await?;

    if res.status().is_success() {
        println!("Status code: {}", res.status());

        let body = res.text().await?;
        let cleaned_html = remove_unuseful_chars(&body);
        save_data_to_file(&cleaned_html, OUTPUT_HTML_PATH, "HTML")?;

        Ok(cleaned_html)
    } else {
        println!("Request was not successful: {}", res.status());
        Ok("".to_string())
    }
}

async fn request(link: &str) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let res = client.get(link).send().await?;
    Ok(res)
}

fn save_data_to_file(data: &str, file_path: &str, file_type: &str) -> Result<(), io::Error> {
    fs::write(file_path, data)?;
    println!("Data saved to {}: {}", file_path, file_type);
    Ok(())
}

fn html_to_json(html: &str) -> Result<Value, Box<dyn Error>> {
    let document = Html::parse_document(&html);
    let selector = Selector::parse("table").unwrap();
    let is_table = document.select(&selector).next().is_some();

    if is_table {
        let mut table_data: Vec<HashMap<String, String>> = vec![];
        let mut column_map: HashMap<usize, Vec<String>> = HashMap::new();

        let row_selector = Selector::parse("table tr").unwrap();

        for row in document.select(&row_selector) {
            let mut col_idx = 0;
            let mut row_data = HashMap::new();

            let cell_selector = Selector::parse("td, th").unwrap();
            row.select(&cell_selector).for_each(|cell| {
                let text = cell.text().collect::<String>();
                row_data.insert(format!("col_{}", col_idx), text.clone());

                let column_entries = column_map.entry(col_idx).or_insert(Vec::new());
                column_entries.push(text);
                col_idx += 1;
            });

            table_data.push(row_data);
        }

        for (_, column) in column_map.iter_mut() {
            column.sort();
        }

        let mut sorted_table_data: Vec<HashMap<String, String>> = vec![];
        let num_columns = column_map.len();

        if num_columns > 0 {
            for row in 0..table_data.len() {
                let mut sorted_row: HashMap<String, String> = HashMap::new();
                for col_idx in 0..num_columns {
                    if let Some(column) = column_map.get(&col_idx) {
                        if row < column.len() {
                            sorted_row.insert(format!("col_{}", col_idx), column[row].clone());
                        } else {
                            sorted_row.insert(format!("col_{}", col_idx), "".to_string());
                        }
                    }
                }
                sorted_table_data.push(sorted_row);
            }
        }

        let result = serde_json::json!({
            "data": sorted_table_data,
            "type": "table"
        });

        Ok(result)
    } else {
        let error_result = serde_json::json!({ "error": "Input is not a table" });
        Ok(error_result)
    }
}

fn html_to_csv(html: &str) -> Result<(), Box<dyn Error>> {
    let document = Html::parse_document(&html);
    let selector = Selector::parse("table").unwrap();
    let table = document.select(&selector).next().unwrap();

    let mut wtr = Writer::from_path("output.csv")?;

    let headers = table
        .select(&Selector::parse("tr").unwrap())
        .next()
        .unwrap()
        .select(&Selector::parse("th, td").unwrap())
        .map(|cell| cell.text().collect::<String>())
        .collect::<Vec<String>>();

    wtr.write_record(&headers)?;

    for row in table.select(&Selector::parse("tr").unwrap()).skip(1) {
        let mut record = Vec::new();
        for cell in row.select(&Selector::parse("th, td").unwrap()) {
            record.push(cell.text().collect::<String>());
        }
        wtr.write_record(&record)?;
    }
    wtr.flush()?;
    Ok(())
}

fn html_to_xml(html: &str) -> Result<(), Box<dyn Error>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("table").unwrap();
    let table = document
        .select(&selector)
        .next()
        .ok_or("No table found in the HTML")?;

    let config = EmitterConfig::new().perform_indent(true);
    let mut writer = config.create_writer(Vec::<u8>::new());

    writer.write(XmlEvent::start_element("table"))?;

    for row in table.select(&Selector::parse("tr").unwrap()) {
        writer.write(XmlEvent::start_element("tr"))?;

        for cell in row.select(&Selector::parse("th, td").unwrap()) {
            let name = cell.value().name();
            writer.write(XmlEvent::start_element(name))?;
            writer.write(XmlEvent::characters(
                cell.text().collect::<String>().as_str(),
            ))?;
            writer.write(XmlEvent::end_element())?;
        }

        writer.write(XmlEvent::end_element())?;
    }

    writer.write(XmlEvent::end_element())?;

    let result = writer.into_inner();
    let mut output_file = fs::File::create("output.xml")?;
    output_file.write_all(&result)?;

    Ok(())
}

fn html_to_toml(html: &str) -> Result<(), Box<dyn Error>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("table").unwrap();
    let table = document
        .select(&selector)
        .next()
        .ok_or("No table found in the HTML")?;

    let mut output = String::new();

    for row in table.select(&Selector::parse("tr").unwrap()) {
        output.push_str("[]\n");

        let mut book_data = String::new();
        for cell in row.select(&Selector::parse("th, td").unwrap()) {
            let name = cell.value().name();
            let content = cell.text().collect::<String>();
            book_data.push_str(&format!("{} = \"{}\"\n", name, content));
        }

        output.push_str(&book_data);
    }

    fs::write("output.toml", output)?;

    Ok(())
}

fn html_to_yaml(html: &str) -> Result<(), Box<dyn Error>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("table").unwrap();
    let table = document
        .select(&selector)
        .next()
        .ok_or("No table found in the HTML")?;

    let mut output = String::new();

    for row in table.select(&Selector::parse("tr").unwrap()) {
        output.push_str("-\n");

        let mut book_data = String::new();
        for cell in row.select(&Selector::parse("th, td").unwrap()) {
            let name = cell.value().name();
            let content = cell.text().collect::<String>();
            book_data.push_str(&format!("{}: \"{}\"\n", name, content));
        }

        output.push_str(&book_data);
    }

    fs::write("output.yaml", output)?;

    Ok(())
}

#[derive(Serialize)]
struct TableRow {
    cells: Vec<TableCell>,
}

#[derive(Serialize)]
struct TableCell {
    name: String,
    content: String,
}

fn html_to_msgpack(html: &str) -> Result<(), Box<dyn Error>> {
    let document = Document::from(html);
    let table = document
        .find(Name("table"))
        .next()
        .ok_or("No table found in the HTML")?;

    let mut table_data = vec![];

    for row in table.find(Name("tr")) {
        let mut cells = vec![];
        for cell in row.find(Name("th, td")) {
            let name = cell.name();
            let content = cell.text();

            let cell_data = TableCell {
                name: name.expect("REASON").to_string(),
                content,
            };
            cells.push(cell_data);
        }

        let row_data = TableRow { cells };
        table_data.push(row_data);
    }

    let packed = rmp_serde::to_vec(&table_data)?;

    std::fs::write("output.msgpack", packed)?;

    Ok(())
}

fn remove_unuseful_chars(text: &str) -> String {
    let mut cleaned_text = text
        .replace("\n", " ")
        .replace("\t", " ")
        .replace("\r", " ");

    let re1 = Regex::new(r"\{[^{}]*\}").unwrap();
    cleaned_text = re1.replace_all(&cleaned_text, " ").to_string();

    let re2 = Regex::new(r"css[^{}]*hover").unwrap();
    cleaned_text = re2.replace_all(&cleaned_text, " ").to_string();

    cleaned_text = Regex::new(r" +")
        .unwrap()
        .replace_all(&cleaned_text, " ")
        .to_string();

    cleaned_text
}

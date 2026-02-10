// ==========================================
// 热轧精整排产系统 - 文件解析器实现
// ==========================================
// 依据: 设计冻结文档 - 阶段 0: 文件读取与解析
// 支持: Excel (.xlsx/.xls) / CSV (.csv)
// ==========================================

use crate::importer::error::ImportError;
use crate::importer::material_importer_trait::FileParser;
use calamine::{open_workbook, Reader, Xlsx};
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

// ==========================================
// CSV Parser 实现
// ==========================================
pub struct CsvParser;

impl FileParser for CsvParser {
    fn parse_to_raw_records(
        &self,
        file_path: &Path,
    ) -> Result<Vec<HashMap<String, String>>, Box<dyn std::error::Error>> {
        let path = file_path;

        // 检查文件存在
        if !path.exists() {
            return Err(Box::new(ImportError::FileNotFound(
                path.display().to_string(),
            )));
        }

        // 检查扩展名
        if let Some(ext) = path.extension() {
            if ext != "csv" {
                return Err(Box::new(ImportError::UnsupportedFormat(
                    ext.to_string_lossy().to_string(),
                )));
            }
        }

        // 打开 CSV 文件
        let file = File::open(path)?;
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true) // 允许行长度不一致
            .from_reader(file);

        // 读取表头
        let headers: Vec<String> = reader
            .headers()?
            .iter()
            .map(|h| h.trim().to_string())
            .collect();

        // 读取所有行
        let mut records = Vec::new();
        for (_row_idx, result) in reader.records().enumerate() {
            let record = result?;
            let mut row_map = HashMap::new();

            for (col_idx, value) in record.iter().enumerate() {
                if let Some(header) = headers.get(col_idx) {
                    row_map.insert(header.clone(), value.trim().to_string());
                }
            }

            // 跳过完全空白的行
            if row_map.values().all(|v| v.is_empty()) {
                continue;
            }

            records.push(row_map);
        }

        Ok(records)
    }
}

// ==========================================
// Excel Parser 实现
// ==========================================
pub struct ExcelParser;

impl FileParser for ExcelParser {
    fn parse_to_raw_records(
        &self,
        file_path: &Path,
    ) -> Result<Vec<HashMap<String, String>>, Box<dyn std::error::Error>> {
        let path = file_path;

        // 检查文件存在
        if !path.exists() {
            return Err(Box::new(ImportError::FileNotFound(
                path.display().to_string(),
            )));
        }

        // 检查扩展名
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "xlsx" && ext != "xls" {
            return Err(Box::new(ImportError::UnsupportedFormat(ext.to_string())));
        }

        // 打开 Excel 文件
        let mut workbook: Xlsx<_> = open_workbook(path)
            .map_err(|e: calamine::XlsxError| ImportError::ExcelParseError(e.to_string()))?;

        // 读取第一个 sheet
        let sheet_names = workbook.sheet_names();
        if sheet_names.is_empty() {
            return Err(Box::new(ImportError::ExcelParseError(
                "Excel 文件无工作表".to_string(),
            )));
        }

        let sheet_name = sheet_names[0].clone();
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| ImportError::ExcelParseError(e.to_string()))?;

        // 提取表头（第一行）
        let mut rows = range.rows();
        let header_row = rows
            .next()
            .ok_or_else(|| ImportError::ExcelParseError("Excel 文件无数据行".to_string()))?;

        let headers: Vec<String> = header_row
            .iter()
            .map(|cell| cell.to_string().trim().to_string())
            .collect();

        // 读取数据行
        let mut records = Vec::new();
        for data_row in rows {
            let mut row_map = HashMap::new();

            for (col_idx, cell) in data_row.iter().enumerate() {
                if let Some(header) = headers.get(col_idx) {
                    let value = cell.to_string().trim().to_string();
                    row_map.insert(header.clone(), value);
                }
            }

            // 跳过完全空白的行
            if row_map.values().all(|v| v.is_empty()) {
                continue;
            }

            records.push(row_map);
        }

        Ok(records)
    }
}

// ==========================================
// 通用文件解析器（根据扩展名自动选择）
// ==========================================
pub struct UniversalFileParser;

impl UniversalFileParser {
    pub fn parse<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> Result<Vec<HashMap<String, String>>, Box<dyn std::error::Error>> {
        let path = file_path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "csv" => {
                let parser = CsvParser;
                parser.parse_to_raw_records(path)
            }
            "xlsx" | "xls" => {
                let parser = ExcelParser;
                parser.parse_to_raw_records(path)
            }
            _ => Err(Box::new(ImportError::UnsupportedFormat(ext))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_parser_valid_file() {
        // 创建临时 CSV 文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "材料号,重量,机组").unwrap();
        writeln!(temp_file, "MAT001,2.5,H032").unwrap();
        writeln!(temp_file, "MAT002,3.0,H033").unwrap();

        let parser = CsvParser;
        let records = parser.parse_to_raw_records(temp_file.path()).unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].get("材料号"), Some(&"MAT001".to_string()));
        assert_eq!(records[0].get("重量"), Some(&"2.5".to_string()));
    }

    #[test]
    fn test_csv_parser_file_not_found() {
        let parser = CsvParser;
        let result = parser.parse_to_raw_records(Path::new("non_existent.csv"));
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_parser_skip_empty_rows() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "材料号,重量").unwrap();
        writeln!(temp_file, "MAT001,2.5").unwrap();
        writeln!(temp_file, ",").unwrap(); // 空行
        writeln!(temp_file, "MAT002,3.0").unwrap();

        let parser = CsvParser;
        let records = parser.parse_to_raw_records(temp_file.path()).unwrap();

        // 应跳过空行
        assert_eq!(records.len(), 2);
    }
}

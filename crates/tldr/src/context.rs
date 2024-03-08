use crate::{
    error::TldrError,
    grammar::ast::{Ast, DataTypeDescriptor, LoadableFormatData},
};

use arrow_csv::infer_schema_from_files;
use datafusion::{datasource::MemTable, execution::context::SessionContext, sql::TableReference};

use arrow::{
    csv::ReaderBuilder,
    datatypes::{DataType, Field, Schema},
};
use std::{ffi::OsStr, fs::File, path::Path, sync::Arc};

pub struct TableColumn<'a> {
    pub table: &'a str,
    pub column: &'a str,
}

pub struct Context {
    pub ctx: SessionContext,
}

impl<'a> Context {
    pub fn convert_ast(ast: &'a Ast) -> Result<Context, TldrError> {
        let ctx = load_base_tables(&ast.loadable_filenames)?;

        Ok(Context { ctx })
    }
}

// load csv, parquet, and json tables...
fn load_base_tables(
    loadable_filenames: &Vec<LoadableFormatData>,
) -> Result<SessionContext, TldrError> {
    let ret = SessionContext::new();

    for filename in loadable_filenames {
        if let LoadableFormatData::CSV(data) = filename {
            let path = Path::new(&data.filename);
            if !path.exists() {
                let s = format!("{}", path.display());
                return Err(TldrError::TldrFileNotfound(s));
            }
            if path.extension() == Some(OsStr::new("csv"))
                || path.extension() == Some(OsStr::new("CSV"))
            {
                println!("reading file: {}", path.display());

                let schema = infer_schema_from_files(
                    &[data.filename.clone()],
                    data.delimiter,
                    data.max_read_records,
                    data.has_header,
                );
                if schema.is_err() {
                    return Err(TldrError::TldrCouldNotReadSchema(data.filename.clone()));
                }
                let schema = schema.unwrap();

                // get the types right ...
                let mod_schema = Schema::new(
                    data.field_types
                        .iter()
                        .map(|(k, v)| {
                            let dtype = match v {
                                DataTypeDescriptor::Time(_, _)
                                | DataTypeDescriptor::Date(_, _)
                                | DataTypeDescriptor::Datetime(_, _, _, _) => DataType::Utf8,
                                DataTypeDescriptor::UInt8(_) => DataType::UInt8,
                                DataTypeDescriptor::UInt16(_) => DataType::UInt16,
                                DataTypeDescriptor::UInt32(_) => DataType::UInt32,
                                DataTypeDescriptor::UInt64(_) => DataType::UInt64,
                                DataTypeDescriptor::Int8(_) => DataType::Int8,
                                DataTypeDescriptor::Int16(_) => DataType::Int16,
                                DataTypeDescriptor::Int32(_) => DataType::Int32,
                                DataTypeDescriptor::Int64(_) => DataType::Int64,
                                DataTypeDescriptor::Float32(_) => DataType::Float32,
                                DataTypeDescriptor::Float64(_) => DataType::Float64,
                                DataTypeDescriptor::String(_) => DataType::Utf8,
                                DataTypeDescriptor::Binary(_) => DataType::Binary,
                                DataTypeDescriptor::Duration(_, tu) => {
                                    DataType::Duration(tu.clone())
                                }
                                DataTypeDescriptor::Boolean(_) => DataType::Boolean,
                                DataTypeDescriptor::Null => DataType::Null,
                            };
                            Field::new(k, dtype, v.is_nullable())
                        })
                        .collect::<Vec<_>>(),
                );

                let schema = Schema::try_merge([schema, mod_schema]);
                if schema.is_err() {
                    return Err(TldrError::TldrCouldNotMergeSchemas(data.filename.clone()));
                }

                let schema = Arc::new(schema.unwrap());
                let file = File::open(path).unwrap();
                let csv_reader = ReaderBuilder::new(schema.clone()).build(file).unwrap();

                let mut batches = Vec::new();
                for batch in csv_reader {
                    if batch.is_err() {
                        return Err(TldrError::TldrCouldNotReadFile(data.filename.clone()));
                    }
                    let batch = batch.unwrap();
                    batches.push(batch);
                }
                let m = MemTable::try_new(schema, vec![batches])
                    .map_err(|_| TldrError::TldrCouldNotCreateMemTable(data.filename.clone()))?;

                ret.register_table(
                    TableReference::bare(path.file_stem().unwrap().to_str().unwrap()),
                    Arc::new(m),
                )
                .map_err(|_| TldrError::TldrCouldNotRegisterTable(data.filename.clone()))?;

                // TODO: Cast Date and Time types into the proper type
            }
        }

        /*         if path.extension() == Some(OsStr::new("parquet")) {
            let file = std::fs::File::open(path);
            if file.is_err() {
                let s = format!("{}", path.display());
                return Err(PlbiError::PlbiCouldNotReadFile(s));
            }
            let df = ParquetReader::new(&mut file.unwrap()).finish();
            if df.is_err() {
                let s = format!("{}", path.display());
                return Err(PlbiError::PlbiCouldNotReadFile(s));
            }
            ret.push(df.unwrap());

            continue;
        }

        if path.extension() == Some(OsStr::new("json")) {
            let file = std::fs::File::open(path);
            if file.is_err() {
                let s = format!("{}", path.display());
                return Err(PlbiError::PlbiCouldNotReadFile(s));
            }

            let df = JsonReader::new(&mut file.unwrap()).finish();
            if df.is_err() {
                let s = format!("{}", path.display());
                return Err(PlbiError::PlbiCouldNotReadFile(s));
            }
            ret.push(df.unwrap());

            continue;
        }
        */
    }

    Ok(ret)
}

#[test]
fn generate_context_test() {
    use crate::grammar::ast::*;
    use std::collections::HashMap;

    let mut online_sales_field_types = HashMap::new();
    online_sales_field_types.insert(
        "SalesOrderNumber".to_string(),
        DataTypeDescriptor::String(false),
    );

    let ast = Ast {
        loadable_filenames: vec![
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimAccount.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimChannel.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimCurrency.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimCustomer.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimDate.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimEmployee.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimEntity.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimGeography.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimMachine.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimOutage.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimProduct.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimProductCategory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimProductSubcategory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimPromotion.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimSalesTerritory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimScenario.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimStore.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactExchangeRate.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactInventory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactITMachine.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactITSLA.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactOnlineSales.csv".to_string(),
                separator: None,
                field_types: online_sales_field_types,
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactSales.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactSalesQuota.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactStrategyPlan.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
        ],
    };

    assert!(Context::convert_ast(&ast).is_ok());
}

#[test]
fn datetime_format_test() {
    use crate::grammar::{ast::*, parser::ast_parser};
    use arrow::datatypes::TimeUnit;
    use std::collections::HashMap;

    let string_to_parse = "load_files 
    CSV(file_name = \"contoso/FactITSLA.csv\", field_types{ (\"OutageStartTime\": Datetime \"%Y-%m-%d %H:%M:%S\" Nanoseconds) (\"OutageEndTime\": Datetime \"%Y-%m-%d %H:%M:%S\" Nanoseconds ) })
    ";

    let parse_result = ast_parser(string_to_parse);

    assert!(parse_result.is_ok());

    let mut dim_date_field_types = HashMap::new();
    dim_date_field_types.insert(
        "OutageStartTime".to_string(),
        DataTypeDescriptor::Datetime(false, "%Y-%m-%d %H:%M:%S", TimeUnit::Nanosecond, None),
    );
    dim_date_field_types.insert(
        "OutageEndTime".to_string(),
        DataTypeDescriptor::Datetime(false, "%Y-%m-%d %H:%M:%S", TimeUnit::Nanosecond, None),
    );

    let expected_ast = Ast {
        loadable_filenames: vec![LoadableFormatData::CSV(CSVData {
            filename: "contoso/FactITSLA.csv".to_string(),
            separator: None,
            field_types: dim_date_field_types,
            delimiter: (";".as_bytes())[0],
            max_read_records: Some(100),
            has_header: true,
        })],
    };

    assert_eq!(parse_result, Ok(("", expected_ast)));

    let (_, ast) = parse_result.unwrap();
    assert!(Context::convert_ast(&ast).is_ok());
}

#[test]
fn date_format_test() {
    use crate::grammar::{ast::*, parser::ast_parser};
    use std::collections::HashMap;

    let string_to_parse = "load_files 
    CSV(file_name = \"contoso/DimDate.csv\", field_types{ (\"DateKey\": Date \"%Y-%m-%d\") })
    ";

    let parse_result = ast_parser(string_to_parse);

    assert!(parse_result.is_ok());

    let mut dim_date_field_types = HashMap::new();
    dim_date_field_types.insert(
        "DateKey".to_string(),
        DataTypeDescriptor::Date(false, "%Y-%m-%d"),
    );

    let expected_ast = Ast {
        loadable_filenames: vec![LoadableFormatData::CSV(CSVData {
            filename: "contoso/DimDate.csv".to_string(),
            separator: None,
            field_types: dim_date_field_types,
            delimiter: (";".as_bytes())[0],
            max_read_records: Some(100),
            has_header: true,
        })],
    };

    assert_eq!(parse_result, Ok(("", expected_ast)));

    let (_, ast) = parse_result.unwrap();
    assert!(Context::convert_ast(&ast).is_ok());
}

#[test]
fn parse_to_context_test() {
    use crate::grammar::{ast::*, parser::ast_parser};
    use std::collections::HashMap;

    let string_to_parse = "load_files 
    CSV(file_name = \"contoso/DimAccount.csv\")
    CSV(file_name = \"contoso/DimChannel.csv\")
    CSV(file_name = \"contoso/DimCurrency.csv\")
    CSV(file_name = \"contoso/DimCustomer.csv\")
    CSV(file_name = \"contoso/DimDate.csv\")
    CSV(file_name = \"contoso/DimEmployee.csv\")
    CSV(file_name = \"contoso/DimEntity.csv\")
    CSV(file_name = \"contoso/DimGeography.csv\")
    CSV(file_name = \"contoso/DimMachine.csv\")
    CSV(file_name = \"contoso/DimOutage.csv\")
    CSV(file_name = \"contoso/DimProduct.csv\")
    CSV(file_name = \"contoso/DimProductCategory.csv\")
    CSV(file_name = \"contoso/DimProductSubcategory.csv\")
    CSV(file_name = \"contoso/DimPromotion.csv\")
    CSV(file_name = \"contoso/DimSalesTerritory.csv\")
    CSV(file_name = \"contoso/DimScenario.csv\")
    CSV(file_name = \"contoso/DimStore.csv\")
    CSV(file_name = \"contoso/FactExchangeRate.csv\")
    CSV(file_name = \"contoso/FactInventory.csv\")
    CSV(file_name = \"contoso/FactITMachine.csv\")
    CSV(file_name = \"contoso/FactITSLA.csv\")
    CSV(file_name = \"contoso/FactOnlineSales.csv\", field_types{ (\"SalesOrderNumber\": String) })
    CSV(file_name = \"contoso/FactSales.csv\")
    CSV(file_name = \"contoso/FactSalesQuota.csv\")
    CSV(file_name = \"contoso/FactStrategyPlan.csv\")
    ";

    let parse_result = ast_parser(string_to_parse);

    assert!(parse_result.is_ok());

    let mut online_sales_field_types = HashMap::new();
    online_sales_field_types.insert(
        "SalesOrderNumber".to_string(),
        DataTypeDescriptor::String(false),
    );

    let mut dim_date_field_types = HashMap::new();
    dim_date_field_types.insert(
        "DateKey".to_string(),
        DataTypeDescriptor::Date(false, "%Y-%m-%d"),
    );

    let expected_ast = Ast {
        loadable_filenames: vec![
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimAccount.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimChannel.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimCurrency.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimCustomer.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimDate.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimEmployee.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimEntity.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimGeography.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimMachine.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimOutage.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimProduct.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimProductCategory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimProductSubcategory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimPromotion.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimSalesTerritory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimScenario.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/DimStore.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactExchangeRate.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactInventory.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactITMachine.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactITSLA.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactOnlineSales.csv".to_string(),
                separator: None,
                field_types: online_sales_field_types,
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactSales.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactSalesQuota.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
            LoadableFormatData::CSV(CSVData {
                filename: "contoso/FactStrategyPlan.csv".to_string(),
                separator: None,
                field_types: HashMap::new(),
                delimiter: (";".as_bytes())[0],
                max_read_records: Some(100),
                has_header: true,
            }),
        ],
    };

    assert_eq!(parse_result, Ok(("", expected_ast)));

    let (_, ast) = parse_result.unwrap();
    assert!(Context::convert_ast(&ast).is_ok());
}

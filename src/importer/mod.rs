// ==========================================
// 热轧精整排产系统 - 导入层
// ==========================================
// 依据: Engine_Specs_v0.3_Integrated.md - 1.1 计算主流程
// ==========================================
// 职责: 外部数据导入,生成内部数据
// 支持: Excel, CSV, 数据库
// ==========================================

// 模块声明
pub mod conflict_handler;
pub mod data_cleaner;
pub mod derivation;
pub mod dq_validator;
pub mod error;
pub mod field_mapper;
pub mod file_parser;
pub mod material_importer;
pub mod material_importer_impl;
pub mod material_importer_trait;

// 重导出核心类型
pub use conflict_handler::ConflictHandler as ConflictHandlerImpl;
pub use data_cleaner::DataCleaner as DataCleanerImpl;
pub use derivation::DerivationService as DerivationServiceImpl;
pub use dq_validator::DqValidator as DqValidatorImpl;
pub use error::{ImportError, ImportResult};
pub use field_mapper::FieldMapper as FieldMapperImpl;
pub use file_parser::{CsvParser, ExcelParser, UniversalFileParser};
pub use material_importer_impl::MaterialImporterImpl;

// 重导出 Trait 接口
pub use material_importer_trait::{
    ConflictHandler, DataCleaner, DerivationService, DqValidator, FieldMapper, FileParser,
    MaterialImporter,
};

// TODO: 添加产能池导入器
// TODO: 添加配置导入器
// TODO: 添加数据映射配置

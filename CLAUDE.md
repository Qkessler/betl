# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

betl (Bank Extract To Ledger) is a Rust CLI tool that parses bank statement files (XLS/XLSX/CSV) and converts them to ledger format for personal finance management. It supports multiple Spanish banks: Santander, CaixaBank (Bankia), Revolut, EvoBank, and Bankinter.

## Development Commands

### Build
- `cargo build` - Build in debug mode
- `cargo build --release` - Build optimized release version
- `cargo check` - Quick compile check without producing executable
- `cargo test` - Run tests (as specified in dyncomp.json)
- `cargo test -v` - Run tests with verbose output

### Run
- `cargo run -- -b <bank> -f <file>` - Run with bank and file arguments
- `cargo run -- -b santander -f /path/to/export.xls` - Example usage
- `cargo run -- --help` - Show help

## Architecture

### Core Components

**Transaction Processing Pipeline:**
1. **Input**: Excel (XLS/XLSX) or CSV files from different banks
2. **Parsing**: Bank-specific parsers handle different file formats and structures
3. **Conversion**: Standardize transactions to internal `Transaction` struct
4. **Output**: Generate ledger-format files

**Key Modules:**
- `main.rs` - CLI interface, configuration parsing, and orchestration
- `bank_statement.rs` - Core traits and implementations for parsing different file types
- `banks.rs` - Bank enumeration and configuration structures
- `transaction_converter.rs` - (Referenced but not examined in detail)

### Bank Support Architecture

Each bank has specific configuration in `main.rs`:
- **Headers**: Column names expected in the file
- **Skip rows**: Number of header/banner rows to skip
- **Sheet name**: Excel worksheet name containing transactions
- **Base account**: Default ledger account for transactions

**Supported Banks:**
- **Santander**: XLS format, "Movimientos" sheet, skips 7 rows
- **CaixaBank/Bankia**: XLS format, uses filename as sheet name, skips 2 rows
- **Revolut**: CSV format, uses filename as sheet name
- **EvoBank**: XLS format, "Movimientos" sheet, skips 1 row
- **Bankinter**: XLSX format, "Movimientos" sheet, skips 8 rows

### Transaction Mapping

The tool supports regex-based transaction description mapping via `~/.config/betl.json`:
```json
{
  "mappings": {
    "regex_pattern": "Expenses:Category"
  }
}
```

### Design Patterns

- **Trait-based parsing**: `BankStatement` trait allows different file format implementations
- **Type-safe configuration**: `BankConfig` struct encapsulates bank-specific settings
- **Generic parsers**: `ExcelBankStatement<ExcelType>` handles both XLS and XLSX
- **Command pattern**: CLI args drive the parsing and conversion flow

## File Structure

- Bank configurations and constants are centralized in `main.rs`
- Parsing logic is abstracted through traits in `bank_statement.rs`
- Each bank type maps to specific parsing implementations
- Output always generates `.ledger` files alongside console output
use std::path::Path;
use std::path::PathBuf;

use addr2line::Location as Addr2LineLocation;
use wasm_bindgen::prelude::*;
use wasm_tools::addr2line::Addr2lineModules;
use wat::GenerateDwarf;
use wat::Parser;

mod error;

#[wasm_bindgen]
#[derive(Debug)]
pub struct ParseError(error::WatParseError);

#[wasm_bindgen]
impl ParseError {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn context(&self) -> String {
        let Self(reason) = self;
        format!("{reason:?}")
    }
}

#[wasm_bindgen]
pub struct Error(error::Error);

#[wasm_bindgen]
impl Error {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn context(&self) -> String {
        let Self(reason) = self;
        format!("{reason:?}")
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Mapping {
    address: u64,
    range_size: u64,
    location: Location,
}

#[wasm_bindgen]
impl Mapping {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn address(&self) -> u64 {
        self.address
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn range_size(&self) -> u64 {
        self.range_size
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn file(&self) -> Option<String> {
        self.location.file.clone()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn line(&self) -> Option<u32> {
        self.location.line
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn column(&self) -> Option<u32> {
        self.location.column
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Location {
    /// The file name.
    file: Option<String>,
    /// The line number.
    line: Option<u32>,
    /// The column number.
    ///
    /// A value of `Some(0)` indicates the left edge.
    column: Option<u32>,
}

#[wasm_bindgen]
impl Location {
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn file(&self) -> Option<String> {
        self.file.clone()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn column(&self) -> Option<u32> {
        self.column
    }
}

impl From<Addr2LineLocation<'_>> for Location {
    fn from(value: Addr2LineLocation<'_>) -> Self {
        Self {
            file: value.file.map(ToString::to_string),
            line: value.line,
            column: value.column,
        }
    }
}

#[wasm_bindgen]
pub struct MappedModule(Vec<u8>);

#[wasm_bindgen]
impl MappedModule {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let Self(bytes) = self;
        bytes.clone()
    }

    /// # Errors
    /// In the case parsing fails, cf. <Error> on retrieving the error info.
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)] // reason: wasm_bindgen + lifetimes
    pub fn from_wat(path: Option<String>, wat: &str) -> Result<Self, ParseError> {
        Self::from_wat_private(path.as_deref().map(PathBuf::from).as_deref(), wat)
            .map_err(ParseError)
    }

    /// # Errors
    /// In the case mapping fails, cf. <Error> on retrieving the error info.
    #[wasm_bindgen]
    pub fn addr2line(self, byte_offset: u64) -> Result<Location, Error> {
        self.addr2line_private(byte_offset).map_err(Error)
    }

    /// # Errors
    /// In the case mapping fails, cf. <Error> on retrieving the error info.
    #[wasm_bindgen]
    pub fn addr2line_mappings(self) -> Result<Vec<Mapping>, Error> {
        self.mappings_private().map_err(Error)
    }
}

/// Macro to append the current file, line and column to a `&'static str`
/// Example: "src/lib.rs:167:58"
macro_rules! location {
    () => {
        concat!(file!(), ":", line!(), ":", column!())
    };
}

pub(crate) struct CodeSectionInformation {
    pub(crate) start_offset: usize,
    pub(crate) size: u32,
}

impl MappedModule {
    fn from_wat_private(path: Option<&Path>, wat: &str) -> Result<Self, error::WatParseError> {
        // Configure new parser with Dwarf support
        let mut parser = Parser::new();
        parser.generate_dwarf(GenerateDwarf::Full);

        // Parse the module, yield early if parsing fails
        let wat_module = parser
            .parse_str(path, wat)
            .map_err(|e| error::WatParseError(format!("{e:?}")))?;

        Ok(Self(wat_module))
    }

    fn addr2line_private(&self, byte_offset: u64) -> Result<Location, error::Error> {
        let Self(module) = self;
        let mut addr2line_modules = Addr2lineModules::parse(module)
            .map_err(|reason| error::Error::Wasmparser(reason.to_string()))?;

        let code_section_relative = false;
        let (ctx, text_relative_address) = addr2line_modules
            .context(byte_offset, code_section_relative)
            .map_err(|reason| error::Error::ContextCreation1(reason.to_string()))?
            .ok_or_else(|| error::Error::ContextCreation2(Box::from(location!())))?;

        // Use text_relative_address here, not byte_offset!
        let outcome = ctx
            .find_location(text_relative_address)
            .map_err(|reason| error::Error::FindTextOffset1(reason.to_string()))?
            .ok_or_else(|| error::Error::FindTextOffset2(Box::from(location!())))?;

        Ok(outcome.into())
    }

    fn mappings_private(&self) -> Result<Vec<Mapping>, error::Error> {
        let Self(module) = self;
        let mut addr2line_modules = Addr2lineModules::parse(module)
            .map_err(|reason| error::Error::Wasmparser(reason.to_string()))?;

        let code_section_relative = false;
        let CodeSectionInformation {
            start_offset: code_section_start_offset,
            size: code_section_size,
        } = self.determine_code_section_size().unwrap();
        let (ctx, text_relative_address) = addr2line_modules
            .context(code_section_start_offset as u64, code_section_relative)
            .map_err(|reason| error::Error::ContextCreation1(reason.to_string()))?
            .ok_or_else(|| error::Error::ContextCreation2(Box::from(location!())))?;

        let mut mappings = vec![];

        for (address, range_size, location) in ctx
            .find_location_range(text_relative_address, code_section_size.into())
            .map_err(|reason| error::Error::FindTextOffset1(reason.to_string()))?
        {
            let location: Location = location.into();
            let mapping = Mapping {
                address: code_section_start_offset as u64 + address,
                range_size,
                location,
            };
            mappings.push(mapping);
        }

        Ok(mappings)
    }

    fn determine_code_section_size(&self) -> Result<CodeSectionInformation, error::Error> {
        let Self(module) = self;

        // Parse the module to find valid code offsets
        let parser = wasmparser::Parser::new(0);

        for payload in parser.parse_all(module) {
            let payload = match payload {
                Ok(payload) => payload,
                Err(reason) => return Err(error::Error::Wasmparser(reason.to_string())),
            };

            if let wasmparser::Payload::CodeSectionStart { size, range, .. } = payload {
                let info = CodeSectionInformation {
                    start_offset: range.start,
                    size,
                };
                return Ok(info);
            }
        }

        Err(error::Error::NoCodeSection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_addresses_work() {
        const WAT: &str = r#"
          (module
            (import "even" "even" (func $even (param i32) (result i32)))
            (export "odd" (func $odd))
            (func $odd (param $0 i32) (result i32)
              local.get $0
              i32.eqz
              if
              i32.const 0
              return
              end
              local.get $0
              i32.const 1
              i32.sub
              call $even))
        "#;

        let mapped_module = MappedModule::from_wat(None, WAT).unwrap();
        let mapping = mapped_module.mappings_private();
        println!("{mapping:?}");
    }
}

// Registration for `getrandom` crate to compile to wasm32-unknown-unknown
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    // Create a slice from the raw pointer and fill it with zeros
    let slice = unsafe { core::slice::from_raw_parts_mut(dest, len) };
    slice.fill(0);
    Ok(())
}

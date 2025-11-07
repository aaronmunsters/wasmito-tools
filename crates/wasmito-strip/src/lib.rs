/// Cf. `wasm-tools / strip` for more information
use wasm_encoder::{ComponentSectionId, Encode, RawSection, Section};
use wasmparser::Parser;
use wasmparser::Payload::{ComponentSection, CustomSection, End, ModuleSection, Version};

pub mod error;

pub struct Config {
    /// Remove all custom sections, regardless of name.
    all: bool,

    /// Remove custom sections matching the specified regex.
    to_delete: Vec<String>,
}

impl Config {
    #[must_use]
    pub fn new(all: bool, to_delete: Vec<String>) -> Self {
        Self { all, to_delete }
    }

    /// # Errors
    /// In case a malformed regex is defined or parsing failed
    pub fn strip(&self, module: Vec<u8>) -> Result<Vec<u8>, error::Error> {
        let input = module;
        let to_delete = regex::RegexSet::new(self.to_delete.iter())
            .map_err(|reason| error::Error::RegexFailed(reason.to_string()))?;

        let strip_custom_section = |name: &str| {
            // If explicitly specified, strip everything.
            if self.all {
                return true;
            }

            // If any section was called out by name only delete those sections.
            if !to_delete.is_empty() {
                return to_delete.is_match(name);
            }

            // Finally default strip everything but:
            // * the `name` section
            // * any `component-type` sections
            // * the `dylink.0` section
            name != "name" && !name.starts_with("component-type:") && name != "dylink.0"
        };

        let mut output = Vec::new();
        let mut stack = Vec::new();

        for payload in Parser::new(0).parse_all(&input) {
            let payload = payload
                .map_err(|reason| error::Error::ParsePayloadRead(reason.message().to_string()))?;

            // Track nesting depth, so that we don't mess with inner producer sections:
            match payload {
                Version { encoding, .. } => {
                    output.extend_from_slice(match encoding {
                        wasmparser::Encoding::Component => &wasm_encoder::Component::HEADER,
                        wasmparser::Encoding::Module => &wasm_encoder::Module::HEADER,
                    });
                }
                ModuleSection { .. } | ComponentSection { .. } => {
                    stack.push(core::mem::take(&mut output));
                    continue;
                }
                End { .. } => {
                    let Some(mut parent) = stack.pop() else { break };
                    if output.starts_with(&wasm_encoder::Component::HEADER) {
                        parent.push(ComponentSectionId::Component as u8);
                        output.encode(&mut parent);
                    } else {
                        parent.push(ComponentSectionId::CoreModule as u8);
                        output.encode(&mut parent);
                    }
                    output = parent;
                }
                _ => {}
            }

            if let CustomSection(c) = &payload
                && strip_custom_section(c.name())
            {
                continue;
            }

            if let Some((id, range)) = payload.as_section() {
                RawSection {
                    id,
                    data: &input[range],
                }
                .append_to(&mut output);
            }
        }

        Ok(output)
    }
}

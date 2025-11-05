import { MappedModule } from "./pkg/wasmito_addr2line.js";

const path = undefined;
const module = MappedModule.from_wat(
  path,
  `
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
`,
);

const wasm_magic_bytes = [0x00, 0x61, 0x73, 0x6D];
for (let index; index < 4; index++) {
  console.assert(module.bytes[index] === wasm_magic_bytes[index]);
}

const mappings = module.addr2line_mappings();
for (const mapping of mappings) {
  console.log(`
    ------------------------------------
    mapping.offset: ${mapping.offset}
      ==>
        mapping.address: ${mapping.address}
        mapping.range_size: ${mapping.range_size}
        mapping.file: ${mapping.file}
        mapping.line: ${mapping.line}
        mapping.column: ${mapping.column}
  `);
}

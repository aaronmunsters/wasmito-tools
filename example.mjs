// replace "./pgk/..." with "wasmito-tools" after
// installing through `npm install github:aaronmunsters/wasmito-tools#pkg`
import assert from "node:assert";
import { Module, StripConfig } from "./pkg/wasmito-tools.js";

const path = undefined;
const wasm_module = Module.from_wat(
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
const actual_bytes = wasm_module.bytes();
wasm_magic_bytes.forEach((byte, byte_index) =>
  assert.equal(byte, actual_bytes[byte_index])
);

const mappings = wasm_module.addr2line_mappings();

assert.equal(mappings[5].address(), BigInt(57));
assert.equal(mappings[5].range_size(), BigInt(1));
assert.equal(mappings[5].file(), "./<input>.wat");
assert.equal(mappings[5].line(), 11);
assert.equal(mappings[5].column(), 5);

const module_including_dwarf = wasm_module;
const stripped = new StripConfig(true, []).strip(
  module_including_dwarf.bytes(),
);
assert(stripped.length < module_including_dwarf.bytes().length);

let mappings_with_offsets = wasm_module.addr2line_mappings_with_offsets();
let buffer = `
`;
for (const mapping of mappings_with_offsets) {
  const padded_line = String(mapping.line()).padStart(2, "0");
  const padded_column = String(mapping.column()).padStart(2, "0");
  buffer += `${mapping.file()}:${padded_line}:${padded_column}`;
  for (const i of mapping.instructions()) {
    buffer += ` | 0x${i.address().toString(16)} | ${i.instr()}\n`;
  }
}

const expected_buffer = `
./<input>.wat:06:05 | 0x31 | local.get
./<input>.wat:07:05 | 0x33 | i32.eqz
./<input>.wat:08:05 | 0x34 | if
./<input>.wat:09:05 | 0x36 | i32.const
./<input>.wat:10:05 | 0x38 | return
./<input>.wat:11:05 | 0x39 | end
./<input>.wat:12:05 | 0x3a | local.get
./<input>.wat:13:05 | 0x3c | i32.const
./<input>.wat:14:05 | 0x3e | i32.sub
./<input>.wat:15:05 | 0x3f | call
./<input>.wat:15:05 | 0x41 | end
`;

assert.equal(expected_buffer, buffer);

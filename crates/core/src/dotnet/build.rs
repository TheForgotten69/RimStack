//! Writes small assemblies for tests: the tables `dotnet` reads, with
//! two-byte indexes throughout.

use super::{Col, ASSEMBLY_REF, METHOD_DEF, TABLES};

/// Where the code starts in memory: after the section's .NET header.
const CODE_RVA: u32 = 0x1048;

#[derive(Default)]
pub struct Builder {
    strings: Vec<u8>,
    blobs: Vec<u8>,
    user_strings: Vec<u8>,
    /// Bytes of code, put before the metadata.
    pub code: Vec<u8>,
    rows: Vec<Vec<Vec<u32>>>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            strings: vec![0],
            blobs: vec![0],
            user_strings: vec![0],
            code: Vec::new(),
            rows: vec![Vec::new(); TABLES.len()],
        }
    }

    pub fn string(&mut self, text: &str) -> u32 {
        let at = self.strings.len() as u32;
        self.strings.extend(text.as_bytes());
        self.strings.push(0);
        at
    }

    pub fn blob(&mut self, bytes: &[u8]) -> u32 {
        let at = self.blobs.len() as u32;
        self.blobs.push(bytes.len() as u8);
        self.blobs.extend(bytes);
        at
    }

    /// A text the code uses; returns its place in their heap.
    pub fn user_string(&mut self, text: &str) -> u32 {
        let at = self.user_strings.len() as u32;
        let units: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        self.user_strings.push(units.len() as u8 + 1);
        self.user_strings.extend(units);
        self.user_strings.push(0);
        at
    }

    /// A method of a type of another assembly, as code calls it.
    pub fn member_ref(&mut self, type_ref: u32, name: &str) -> u32 {
        let (name, signature) = (self.string(name), self.blob(&[0x00, 0, 0x01]));
        self.row(super::MEMBER_REF, &[type_ref << 3 | 1, name, signature])
    }

    /// Adds a row and returns its number, from 1.
    pub fn row(&mut self, table: usize, cells: &[u32]) -> u32 {
        assert_eq!(cells.len(), TABLES[table].len());
        self.rows[table].push(cells.to_vec());
        self.rows[table].len() as u32
    }

    pub fn type_ref(&mut self, namespace: &str, name: &str) -> u32 {
        let (name, namespace) = (self.string(name), self.string(namespace));
        self.row(super::TYPE_REF, &[0, name, namespace])
    }

    /// A type that extends a `TypeRef` row (0 for none); its methods
    /// are the `MethodDef` rows added until the next type.
    pub fn type_def(&mut self, namespace: &str, name: &str, extends_ref: u32) -> u32 {
        let (name, namespace) = (self.string(name), self.string(namespace));
        let extends = if extends_ref == 0 {
            0
        } else {
            extends_ref << 2 | 1
        };
        let next_method = self.rows[super::METHOD_DEF].len() as u32 + 1;
        self.row(
            super::TYPE_DEF,
            &[0, name, namespace, extends, 1, next_method],
        )
    }

    pub fn method(&mut self, name: &str, signature: &[u8]) -> u32 {
        let (name, signature) = (self.string(name), self.blob(signature));
        self.row(METHOD_DEF, &[0, 0, 0, name, signature, 1])
    }

    /// A method with these flags and this code, in the small form of a
    /// method body (under 64 bytes).
    pub fn method_with(&mut self, name: &str, flags: u32, code: &[u8]) -> u32 {
        let (name, signature) = (self.string(name), self.blob(&[0x00, 0, 0x01]));
        let rva = CODE_RVA + self.code.len() as u32;
        self.code.push((code.len() as u8) << 2 | 2);
        self.code.extend(code);
        self.row(METHOD_DEF, &[rva, 0, flags, name, signature, 1])
    }

    pub fn field(&mut self, name: &str) -> u32 {
        let (name, signature) = (self.string(name), self.blob(&[0x06, 0x1c]));
        self.row(super::FIELD, &[0, name, signature])
    }

    pub fn implements(&mut self, class: u32, interface: u32) {
        self.row(super::INTERFACE_IMPL, &[class, interface << 2]);
    }

    pub fn nested(&mut self, inner: u32, outer: u32) {
        self.row(super::NESTED_CLASS, &[inner, outer]);
    }

    /// A type of this assembly that extends another of its types
    /// (row 0 for none).
    pub fn type_def_under(&mut self, namespace: &str, name: &str, extends_def: u32) -> u32 {
        let (name, namespace) = (self.string(name), self.string(namespace));
        let next_method = self.rows[METHOD_DEF].len() as u32 + 1;
        self.row(
            super::TYPE_DEF,
            &[0, name, namespace, extends_def << 2, 1, next_method],
        )
    }

    /// A constructor of a type of another assembly, as attributes use.
    pub fn constructor(&mut self, type_ref: u32, signature: &[u8]) -> u32 {
        let (name, signature) = (self.string(".ctor"), self.blob(signature));
        self.row(super::MEMBER_REF, &[type_ref << 3 | 1, name, signature])
    }

    /// Puts an attribute on a type (`on_method` false) or a method.
    pub fn attribute(&mut self, on_method: bool, parent: u32, constructor: u32, value: &[u8]) {
        let parent = parent << 5 | if on_method { 0 } else { 3 };
        let value = self.blob(value);
        self.row(
            super::CUSTOM_ATTRIBUTE,
            &[parent, constructor << 3 | 3, value],
        );
    }

    pub fn reference(&mut self, name: &str) {
        let name = self.string(name);
        self.row(ASSEMBLY_REF, &[0, 0, 0, name, 0, 0]);
    }

    pub fn finish(self) -> Vec<u8> {
        let mut tables = vec![0u8; 24];
        let present: u64 = (0..TABLES.len())
            .filter(|t| !self.rows[*t].is_empty())
            .fold(0, |bits, t| bits | 1 << t);
        tables[8..16].copy_from_slice(&present.to_le_bytes());
        for rows in self.rows.iter().filter(|r| !r.is_empty()) {
            tables.extend((rows.len() as u32).to_le_bytes());
        }
        for (table, rows) in self.rows.iter().enumerate() {
            for row in rows {
                for (cell, col) in row.iter().zip(TABLES[table]) {
                    match col {
                        Col::Fixed(bytes) => {
                            tables.extend(&(*cell as u64).to_le_bytes()[..(*bytes).min(8)]);
                            tables.extend(std::iter::repeat_n(0, bytes.saturating_sub(8)));
                        }
                        _ => tables.extend((*cell as u16).to_le_bytes()),
                    }
                }
            }
        }
        let streams: [(&[u8], &[u8]); 4] = [
            (b"#~\0\0", &tables),
            (b"#Strings\0\0\0\0", &self.strings),
            (b"#Blob\0\0\0", &self.blobs),
            (b"#US\0", &self.user_strings),
        ];
        // Metadata: signature, a four-byte version text, the streams.
        let mut metadata = Vec::new();
        metadata.extend(0x424a_5342u32.to_le_bytes());
        metadata.extend([0u8; 8]);
        metadata.extend(4u32.to_le_bytes());
        metadata.extend([0u8; 4]);
        metadata.extend([0, 0, streams.len() as u8, 0]);
        let heads: usize = streams.iter().map(|(name, _)| 8 + name.len()).sum();
        let mut at = metadata.len() + heads;
        for (name, bytes) in streams {
            metadata.extend((at as u32).to_le_bytes());
            metadata.extend((bytes.len() as u32).to_le_bytes());
            metadata.extend(name);
            at += bytes.len();
        }
        for (_, bytes) in streams {
            metadata.extend(bytes);
        }
        // The least of a Windows program around it: one section, with
        // the .NET header at its start and the metadata after it.
        let mut dll = vec![0u8; 0x248];
        let mut put = |at: usize, bytes: &[u8]| dll[at..at + bytes.len()].copy_from_slice(bytes);
        put(0x3c, &0x40u32.to_le_bytes());
        put(0x40, b"PE\0\0");
        put(0x46, &[1, 0]);
        put(0x54, &[224, 0]);
        put(0x58, &[0x0b, 0x01]);
        put(0x58 + 96 + 14 * 8, &0x1000u32.to_le_bytes());
        put(0x138 + 8, &0x0010_0000u32.to_le_bytes());
        put(0x138 + 12, &0x1000u32.to_le_bytes());
        put(0x138 + 20, &0x200u32.to_le_bytes());
        put(0x208, &(CODE_RVA + self.code.len() as u32).to_le_bytes());
        dll.extend(self.code);
        dll.extend(metadata);
        dll
    }
}

/// An instruction with a token: its one-byte opcode (two bytes after
/// 0xFE for those above 0xFF), then the table and the row.
pub fn op(code: u16, table: usize, row: u32) -> Vec<u8> {
    let mut bytes = match code {
        0..=0xff => vec![code as u8],
        _ => vec![0xfe, code as u8],
    };
    bytes.extend([row as u8, (row >> 8) as u8, 0, table as u8]);
    bytes
}

/// Flags of a method: called through its object, sealed, first declared.
pub const VIRTUAL: u32 = 0x40;
pub const FINAL: u32 = 0x20;
pub const NEW: u32 = 0x100;

/// A text as attribute values hold it.
pub fn ser(text: &str) -> Vec<u8> {
    let mut bytes = vec![text.len() as u8];
    bytes.extend(text.as_bytes());
    bytes
}

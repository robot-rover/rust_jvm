use std::io::Read;
use byteorder::BigEndian;
use constant_pool::cp_info::*;
use byteorder::ReadBytesExt;
use cesu8::from_java_cesu8;
use class_file::ClassLoadingError;
use class::ClassRef;
use class::ClassRef::Symbolic;
use std::ops::Index;
use typed_arena::Arena;

#[derive(Debug)]
pub struct ConstantPool<'a>(Vec<Option<cp_info<'a>>>);

impl<'a> ConstantPool<'a> {
    pub fn get_entry(&self, index: u16) -> &cp_info<'a> {
        self.0.index(index as usize).as_ref().unwrap()
    }

    pub fn get_string_entry(&self, index: u16) -> &'a str {
        match self.get_entry(index) {
            CONSTANT_Utf8_info { bytes } => *bytes,
            other => panic!("Symbolic Class reference in ClassFile#super_class didn't point to CONSTANT_Utf8_info, instead: {:?}", other)
        }
    }
}

#[derive(Debug)]
pub enum cp_info<'a> {
    /// `name_index` -> constant_pool index of a `CONSTANT_Utf8_info` representing classname
    CONSTANT_Class_info {
        name_index: u16
    },

    /// `class_index` -> constant_pool index of a `CONSTANT_Class_info`
    ///
    /// `name_and_type_index` -> constant_pool index of a `CONSTANT_NameAndType_info`
    CONSTANT_Fieldref_info {
        class_index: u16,
        name_and_type_index: u16
    },

    /// `class_index` -> constant_pool index of a `CONSTANT_Class_info`
    ///
    /// `name_and_type_index` -> constant_pool index of a `CONSTANT_NameAndType_info`
    CONSTANT_Methodref_info {
        class_index: u16,
        name_and_type_index: u16
    },

    /// `class_index` -> constant_pool index of a `CONSTANT_Class_info`
    ///
    /// `name_and_type_index` -> constant_pool index of a `CONSTANT_NameAndType_info`
    CONSTANT_InterfaceMethodref_info {
        class_index: u16,
        name_and_type_index: u16
    },

    /// `string_index` -> constant_pool index of a `CONSTANT_utf8_info`
    CONSTANT_String_info {
        string_index: u16
    },

    /// `bytes` -> big-endian representation of an int
    CONSTANT_Integer_info {
        bytes: i32
    },

    /// `bytes` -> representation of an IEEE 754 floating point single number
    CONSTANT_Float_info {
        bytes: f32
    },

    /// `value` -> 64 bit signed integer value
    CONSTANT_Long_info {
        value: i64
    },

    /// `value` ->  IEEE 754 floating point double value
    CONSTANT_Double_info {
        value: f64
    },

    /// `name_index` -> constant_pool index of a `CONSTANT_utf8_info` that is the name of a method or field
    ///
    /// `descriptor_index` -> constant_pool index of a `CONSTANT_utf8_info` that is a method or field descriptor
    CONSTANT_NameAndType_info {
        name_index: u16,
        descriptor_index: u16
    },

    /// `bytes` -> bytes of the string
    CONSTANT_Utf8_info {
        bytes: &'a str
    }
}

pub fn read_constant_pool<'a, 'b>(input: &'b mut Read, constant_pool_count: u16, string_allocator: &'a Arena<String>) -> Result<ConstantPool<'a>, ClassLoadingError> {
    let mut iter = constant_pool_count - 1;
    let mut pool = Vec::with_capacity(constant_pool_count as usize);
    pool.push(Option::None);
    while iter > 0 {
        let info = cp_info::new(input, string_allocator)?;
        match info {
            CONSTANT_Double_info { .. } | CONSTANT_Long_info { .. } => {
                pool.push(Option::Some(info));
                pool.push(Option::None);
                iter -= 2;
            }
            _ => {
                pool.push(Option::Some(info));
                iter -= 1;
            }
        }
    }
    Ok(ConstantPool(pool))
}

impl<'a> cp_info<'a> {
    fn new(input: &mut Read, allocator: &'a Arena<String>) -> Result<cp_info<'a>, ClassLoadingError> {
        let tag = input.read_u8()?;
        Ok(match tag {
            7 => {
                let class_index = input.read_u16::<BigEndian>()?;
                CONSTANT_Class_info { name_index: class_index }
            }
            9 => {
                let class_index = input.read_u16::<BigEndian>()?;
                let name_and_type_index = input.read_u16::<BigEndian>()?;
                CONSTANT_Fieldref_info { class_index, name_and_type_index }
            }
            10 => {
                let class_index = input.read_u16::<BigEndian>()?;
                let name_and_type_index = input.read_u16::<BigEndian>()?;
                CONSTANT_Methodref_info { class_index, name_and_type_index }
            }
            11 => {
                let class_index = input.read_u16::<BigEndian>()?;
                let name_and_type_index = input.read_u16::<BigEndian>()?;
                CONSTANT_InterfaceMethodref_info { class_index, name_and_type_index }
            }
            8 => {
                let string_index = input.read_u16::<BigEndian>()?;
                CONSTANT_String_info { string_index }
            }
            3 => {
                let bytes = input.read_i32::<BigEndian>()?;
                CONSTANT_Integer_info { bytes }
            }
            4 => {
                let bytes = input.read_f32::<BigEndian>()?;

                CONSTANT_Float_info { bytes }
            }
            5 => {
                let value = input.read_i64::<BigEndian>()?;
                CONSTANT_Long_info { value }
            }
            6 => {
                let value = input.read_f64::<BigEndian>()?;
                CONSTANT_Double_info { value }
            }
            12 => {
                let name_index = input.read_u16::<BigEndian>()?;
                let descriptor_index = input.read_u16::<BigEndian>()?;
                CONSTANT_NameAndType_info {name_index, descriptor_index}
            }
            1 => {
                let length = input.read_u16::<BigEndian>()?;
                let mut bytes = vec![0u8; length as usize];
                input.read_exact(&mut bytes)?;
                let string = from_java_cesu8(&bytes)?;
                let reference = allocator.alloc(string.to_string());
                CONSTANT_Utf8_info { bytes: reference.as_str() }
            }
            _ => panic!("Unknown Constant Pool Tag parsed: {}", tag)
        })
    }
}


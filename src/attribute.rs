use std::io::Read;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use attribute::attribute_info_Data::*;
use constant_pool::cp_info::*;
use constant_pool::ConstantPool;
use attribute::stack_map_frame_data::*;
use attribute::verification_type_info_data::*;
use attribute::element_value_data::*;
use class_file::ClassLoadingError;

#[derive(Debug)]
pub struct attribute_info {
    attribute_name_index: u16,
    attribute_length: u32,
    info: attribute_info_Data
}

#[derive(Debug)]
pub enum attribute_info_Data {
    ConstantValue_attribute {
        constantvalue_index: u16
    },

    Code_attribute {
        max_stack: u16,
        max_locals: u16,
        code_length: u32,
        code: Vec<u8>,
        exception_table_length: u16,
        exception_table: Vec<exception_info>,
        attributes_count: u16,
        attributes: Vec<attribute_info>
    },

    StackMapTable_attribute {
        number_of_entries: u16,
        entries: Vec<stack_map_frame>
    },

    Exceptions_attribute {
        number_of_exceptions: u16,
        exception_index_table: Vec<u16>
    },

    InnerClasses_attribute {
        number_of_classes: u16,
        classes: Vec<inner_class>
    },

    EnclosingMethod_attribute {
        class_index: u16,
        method_index: u16
    },

    Synthetic_attribute,

    Signature_attribute {
        signature_index: u16
    },

    SourceFile_attribute {
        sourcefile_index: u16
    },

    SourceDebugExtension {
        debug_extension: Vec<u8>
    },

    LineNumberTable_attribute {
        line_number_table_length: u16,
        line_number_table: Vec<line_number_table_entry>
    },

    LocalVariableTable_attribute {
        local_variable_table_length: u16,
        local_variable_table: Vec<local_variable_table_entry>
    },

    LocalVariableTypeTable_attribute {
        local_variable_type_table_length: u16,
        local_variable_type_table: Vec<local_variable_type_table_entry>
    },

    Deprecated_attribute,

    RuntimeVisibleAnnotations_attribute {
        num_annotations: u16,
        annotations: Vec<annotation>
    },

    RuntimeInvisibleAnnotations_attribute {
        num_annotations: u16,
        annotations: Vec<annotation>
    },

    RuntimeVisibleParameterAnnotations_attribute {
        num_parameters: u8,
        parameter_annotations: Vec<annotation_list>
    },

    RuntimeInvisibleParameterAnnotations_attribute {
        num_parameters: u8,
        parameter_annotations: Vec<annotation_list>
    },

    AnnotationDefault_attribute {
        default_value: element_value
    },

    BootstrapMethods_attribute {
        num_bootstrap_methods: u16,
        bootstrap_methods: Vec<bootstrap_method>
    },

    RuntimeVisibleTypeAnnotations {

    },

    RuntimeInvisibleTypeAnnotations {

    },

    Unknown_attribute {
        info: Vec<u8>
    }
}

#[derive(Debug)]
struct bootstrap_method {
    bootstrap_method_ref: u16,
    num_bootstrap_arguments: u16,
    bootstrap_arguments: Vec<u16>
}

#[derive(Debug)]
struct annotation_list {
    num_annotations: u16,
    annotations: Vec<annotation>
}

#[derive(Debug)]
struct annotation {
    type_index: u16,
    num_element_value_pairs: u16,
    element_value_pairs: Vec<element_value_pair>
}

impl annotation {
    pub fn new(input: &mut Read) -> Result<annotation, ClassLoadingError> {
        let type_index = input.read_u16::<BigEndian>()?;
        let num_element_value_pairs = input.read_u16::<BigEndian>()?;
        let mut element_value_pairs = Vec::with_capacity(num_element_value_pairs as usize);
        for _ in 0..num_element_value_pairs {
            let element_name_index = input.read_u16::<BigEndian>()?;
            let value = element_value::new(input)?;
            element_value_pairs.push(element_value_pair { element_name_index, value })
        }
        Ok(annotation {
            type_index,
            num_element_value_pairs,
            element_value_pairs
        })
    }
}

#[derive(Debug)]
struct element_value_pair {
    element_name_index: u16,
    value: element_value
}

#[derive(Debug)]
struct element_value {
    tag: u8,
    value: element_value_data
}

impl element_value {
    pub fn new(input: &mut Read) -> Result<element_value, ClassLoadingError> {
        let tag = input.read_u8()?;
        let value = match tag as char {
            'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' | 's' => {
                const_value_index(input.read_u16::<BigEndian>()?)
            }
            'e' => {
                let type_name_index = input.read_u16::<BigEndian>()?;
                let const_name_index = input.read_u16::<BigEndian>()?;
                enum_const_value { type_name_index, const_name_index }
            }
            'c' => {
                class_info_index(input.read_u16::<BigEndian>()?)
            }
            '@' => {
                annotation_value(annotation::new(input)?)
            }
            '[' => {
                let num_values = input.read_u16::<BigEndian>()?;
                let mut values = Vec::with_capacity(num_values as usize);
                for _ in 0..num_values {
                    values.push(element_value::new(input)?);
                }
                array_value { num_values, values }
            }
            _ => panic!("Parsed illegal element_value#tag: {}", tag)
        };
        Ok(element_value { tag, value })
    }
}

#[derive(Debug)]
enum element_value_data {
    const_value_index(u16),
    enum_const_value {
        type_name_index: u16,
        const_name_index: u16
    },
    class_info_index(u16),
    annotation_value(annotation),
    array_value {
        num_values: u16,
        values: Vec<element_value>
    }
}

#[derive(Debug)]
struct local_variable_table_entry {
    start_pc: u16,
    length: u16,
    name_index: u16,
    descriptor_index: u16,
    index: u16
}

#[derive(Debug)]
struct local_variable_type_table_entry {
    start_pc: u16,
    length: u16,
    name_index: u16,
    signature_index: u16,
    index: u16
}

#[derive(Debug)]
struct line_number_table_entry {
    start_pc: u16,
    line_number: u16
}

#[derive(Debug)]
struct inner_class {
    inner_class_info_index: u16,
    outer_class_info_index: u16,
    inner_name_index: u16,
    inner_class_access_flags: u16
}

#[derive(Debug)]
struct stack_map_frame {
    frame_type: u8,
    frame_data: stack_map_frame_data
}

impl stack_map_frame {
    fn new(input: &mut Read) -> Result<stack_map_frame, ClassLoadingError> {
        let frame_type = input.read_u8()?;
        let frame_data = match frame_type {
            0..=63 => {
                same_frame
            }
            64..=127 => {
                let stack = verification_type_info::new(input)?;
                same_locals_1_stack_item_frame { stack }
            }
            247 => {
                let offset_delta = input.read_u16::<BigEndian>()?;
                let stack = verification_type_info::new(input)?;
                same_locals_1_stack_item_frame_extended { offset_delta, stack }
            }
            248..=250 => {
                let offset_delta = input.read_u16::<BigEndian>()?;
                chop_frame { offset_delta }
            }
            251 => {
                let offset_delta = input.read_u16::<BigEndian>()?;
                same_frame_extended { offset_delta }
            }
            252..=254 => {
                let offset_delta = input.read_u16::<BigEndian>()?;
                let mut locals = Vec::with_capacity((frame_type-251) as usize);
                for _ in 0..(frame_type-251) {
                    locals.push(verification_type_info::new(input)?);
                }
                append_frame { offset_delta, locals }
            }
            255 => {
                let offset_delta = input.read_u16::<BigEndian>()?;
                let number_of_locals = input.read_u16::<BigEndian>()?;
                let mut locals = Vec::with_capacity(number_of_locals as usize);
                for _ in 0..number_of_locals {
                    locals.push(verification_type_info::new(input)?);
                }
                let number_of_stack_items = input.read_u16::<BigEndian>()?;
                let mut stack = Vec::with_capacity(number_of_stack_items as usize);
                for _ in 0..number_of_stack_items {
                    locals.push(verification_type_info::new(input)?);
                }
                full_frame {
                    offset_delta,
                    number_of_locals,
                    locals,
                    number_of_stack_items,
                    stack
                }
            }
            _ => {
                panic!("Parsed stack_map_frame#frame_type reserved for future use: {}", frame_type);
            }
        };
        Ok(stack_map_frame { frame_type , frame_data })
    }
}

#[derive(Debug)]
enum stack_map_frame_data {
    same_frame,

    same_locals_1_stack_item_frame {
        stack: verification_type_info
    },

    same_locals_1_stack_item_frame_extended {
        offset_delta: u16,
        stack: verification_type_info
    },

    chop_frame {
        offset_delta: u16
    },

    same_frame_extended {
        offset_delta: u16
    },

    append_frame {
        offset_delta: u16,
        locals: Vec<verification_type_info>
    },

    full_frame {
        offset_delta: u16,
        number_of_locals: u16,
        locals: Vec<verification_type_info>,
        number_of_stack_items: u16,
        stack: Vec<verification_type_info>
    }
}

#[derive(Debug)]
struct verification_type_info {
    tag: u8,
    data: verification_type_info_data
}

#[derive(Debug)]
enum verification_type_info_data {
    Top_variable_info,
    Integer_variable_info,
    Float_variable_info,
    Long_variable_info,
    Double_variable_info,
    Null_variable_info,
    UninitializedThis_variable_info,
    Object_variable_info {
        cpool_index: u16
    },
    Uninitialized_variable_info {
        offset: u16
    }
}

impl verification_type_info {
    pub fn new(input: &mut Read) -> Result<verification_type_info, ClassLoadingError> {
        let tag = input.read_u8()?;
        let data = match tag {
            0 => Top_variable_info,
            1 => Integer_variable_info,
            2 => Float_variable_info,
            4 => Long_variable_info,
            3 => Double_variable_info,
            5 => Null_variable_info,
            6 => UninitializedThis_variable_info,
            7 => Object_variable_info { cpool_index: input.read_u16::<BigEndian>()? },
            8 => Uninitialized_variable_info { offset: input.read_u16::<BigEndian>()? },
            _ => panic!("Unsupported verification_type_info#tag parsed: {}", tag)
        };
        Ok(verification_type_info { tag, data })
    }
}

pub fn read_attributes(input: &mut Read, length: u16, constant_pool: &ConstantPool) -> Result<Vec<attribute_info>, ClassLoadingError> {
    let mut vector = Vec::with_capacity(length as usize);
    for _ in 0..length {
        vector.push(attribute_info::new(input, constant_pool)?);
    }
    Ok(vector)
}

impl attribute_info {
    pub fn new(input: &mut Read, constant_pool: &ConstantPool) -> Result<attribute_info, ClassLoadingError> {
        let attribute_name_index = input.read_u16::<BigEndian>()?;
        let attribute_length = input.read_u32::<BigEndian>()?;
        let item = constant_pool.get_entry(attribute_name_index);
        let attribute_name = match item {
            CONSTANT_Utf8_info { bytes, .. } => *bytes,
            _ => panic!("attribute_name pointed to {:#?}, not CONSTANT_Utf8_info", item)
        };

        let info = attribute_info::parse_info(input, constant_pool, attribute_length, attribute_name)?;

        Ok(attribute_info {
            attribute_name_index,
            attribute_length,
            info
        })
    }

    pub fn get_data(&self) -> &attribute_info_Data {
        &self.info
    }

    fn parse_info(input: &mut Read, constant_pool: &ConstantPool, attribute_length: u32, name: &str) -> Result<attribute_info_Data, ClassLoadingError> {
        Ok(match name {
            "ConstantValue" => {
                let constantvalue_index = input.read_u16::<BigEndian>()?;
                ConstantValue_attribute { constantvalue_index }
            }
            "Code" => {
                let max_stack = input.read_u16::<BigEndian>()?;
                let max_locals = input.read_u16::<BigEndian>()?;
                let code_length = input.read_u32::<BigEndian>()?;
                let mut code = vec![0u8; code_length as usize];
                input.read_exact(&mut code)?;
                let exception_table_length = input.read_u16::<BigEndian>()?;
                let mut exception_table = Vec::with_capacity(exception_table_length as usize);
                for _ in 0..exception_table_length {
                    let start_pc = input.read_u16::<BigEndian>()?;
                    let end_pc = input.read_u16::<BigEndian>()?;
                    let handler_pc = input.read_u16::<BigEndian>()?;
                    let catch_type = input.read_u16::<BigEndian>()?;
                    exception_table.push(exception_info {
                        start_pc,
                        end_pc,
                        handler_pc,
                        catch_type
                    })
                }
                let attributes_count = input.read_u16::<BigEndian>()?;
                let mut attributes = Vec::with_capacity(attributes_count as usize);
                for _ in 0..attributes_count {
                    attributes.push(attribute_info::new(input, constant_pool)?);
                }
                Code_attribute {
                    max_stack,
                    max_locals,
                    code_length,
                    code,
                    exception_table_length,
                    exception_table,
                    attributes_count,
                    attributes
                }
            }
            "StackMapTable" => {
                let number_of_entries = input.read_u16::<BigEndian>()?;
                let mut entries = Vec::with_capacity(number_of_entries as usize);
                for _ in 0..number_of_entries {
                    entries.push(stack_map_frame::new(input)?);
                }
                StackMapTable_attribute { number_of_entries, entries }
            }
            "Exceptions" => {
                let number_of_exceptions = input.read_u16::<BigEndian>()?;
                let mut exception_index_table = Vec::with_capacity(number_of_exceptions as usize);
                for _ in 0..number_of_exceptions {
                    exception_index_table.push(input.read_u16::<BigEndian>()?);
                }
                Exceptions_attribute { number_of_exceptions, exception_index_table }

            }
            "InnerClasses" => {
                let number_of_classes = input.read_u16::<BigEndian>()?;
                let mut classes = Vec::with_capacity(number_of_classes as usize);
                for _ in 0..number_of_classes {
                    let inner_class_info_index = input.read_u16::<BigEndian>()?;
                    let outer_class_info_index = input.read_u16::<BigEndian>()?;
                    let inner_name_index = input.read_u16::<BigEndian>()?;
                    let inner_class_access_flags = input.read_u16::<BigEndian>()?;
                    classes.push(inner_class {
                        inner_class_info_index,
                        outer_class_info_index,
                        inner_name_index,
                        inner_class_access_flags
                    });
                }
                InnerClasses_attribute { number_of_classes, classes }
            }
            "EnclosingMethod" => {
                let class_index = input.read_u16::<BigEndian>()?;
                let method_index = input.read_u16::<BigEndian>()?;
                EnclosingMethod_attribute { class_index, method_index }
            }
            "Synthetic" => {
                Synthetic_attribute
            }
            "Signature" => {
                let signature_index = input.read_u16::<BigEndian>()?;
                Signature_attribute { signature_index }
            }
            "SourceFile" => {
                let sourcefile_index = input.read_u16::<BigEndian>()?;
                SourceFile_attribute { sourcefile_index }
            }
            "SourceDebugExtension" => {
                let mut debug_extension = vec![0u8; attribute_length as usize];
                input.read_exact(&mut debug_extension)?;
                SourceDebugExtension { debug_extension }
            }
            "LineNumberTable" => {
                let line_number_table_length = input.read_u16::<BigEndian>()?;
                let mut line_number_table = Vec::with_capacity(line_number_table_length as usize);
                for _ in 0..line_number_table_length {
                    let start_pc = input.read_u16::<BigEndian>()?;
                    let line_number = input.read_u16::<BigEndian>()?;
                    line_number_table.push(line_number_table_entry { start_pc, line_number })
                }
                LineNumberTable_attribute {
                    line_number_table_length,
                    line_number_table
                }
            }
            "LocalVariableTable" => {
                let local_variable_table_length = input.read_u16::<BigEndian>()?;
                let mut local_variable_table = Vec::with_capacity(local_variable_table_length as usize);
                for _ in 0..local_variable_table_length {
                    let start_pc = input.read_u16::<BigEndian>()?;
                    let length = input.read_u16::<BigEndian>()?;
                    let name_index = input.read_u16::<BigEndian>()?;
                    let descriptor_index = input.read_u16::<BigEndian>()?;
                    let index = input.read_u16::<BigEndian>()?;
                    local_variable_table.push(local_variable_table_entry {
                        start_pc,
                        length,
                        name_index,
                        descriptor_index,
                        index
                    })
                }
                LocalVariableTable_attribute { local_variable_table_length, local_variable_table }
            }
            "LocalVariableTypeTable" => {
                let local_variable_type_table_length = input.read_u16::<BigEndian>()?;
                let mut local_variable_type_table = Vec::with_capacity(local_variable_type_table_length as usize);
                for _ in 0..local_variable_type_table_length {
                    let start_pc = input.read_u16::<BigEndian>()?;
                    let length = input.read_u16::<BigEndian>()?;
                    let name_index = input.read_u16::<BigEndian>()?;
                    let signature_index = input.read_u16::<BigEndian>()?;
                    let index = input.read_u16::<BigEndian>()?;
                    local_variable_type_table.push(local_variable_type_table_entry {
                        start_pc,
                        length,
                        name_index,
                        signature_index,
                        index
                    })
                }
                LocalVariableTypeTable_attribute { local_variable_type_table_length, local_variable_type_table }
            }
            "Deprecated" => {
                Deprecated_attribute
            }
            "RuntimeVisibleAnnotations" => {
                let num_annotations = input.read_u16::<BigEndian>()?;
                let mut annotations = Vec::with_capacity(num_annotations as usize);
                for _ in 0..num_annotations {
                    annotations.push(annotation::new(input)?)
                }
                RuntimeVisibleAnnotations_attribute { num_annotations, annotations }
            }
            "RuntimeInvisibleAnnotations" => {
                let num_annotations = input.read_u16::<BigEndian>()?;
                let mut annotations = Vec::with_capacity(num_annotations as usize);
                for _ in 0..num_annotations {
                    annotations.push(annotation::new(input)?)
                }
                RuntimeInvisibleAnnotations_attribute { num_annotations, annotations }
            }
            "RuntimeVisibleParameterAnnotations" => {
                let num_parameters = input.read_u8()?;
                let mut parameter_annotations = Vec::with_capacity(num_parameters as usize);
                for _ in 0..num_parameters {
                    let num_annotations = input.read_u16::<BigEndian>()?;
                    let mut annotations = Vec::with_capacity(num_annotations as usize);
                    for _ in 0..num_annotations {
                        annotations.push(annotation::new(input)?);
                    }
                    parameter_annotations.push(annotation_list { num_annotations, annotations })
                }
                RuntimeVisibleParameterAnnotations_attribute { num_parameters, parameter_annotations }
            }
            "RuntimeInvisibleParameterAnnotations" => {
                let num_parameters = input.read_u8()?;
                let mut parameter_annotations = Vec::with_capacity(num_parameters as usize);
                for _ in 0..num_parameters {
                    let num_annotations = input.read_u16::<BigEndian>()?;
                    let mut annotations = Vec::with_capacity(num_annotations as usize);
                    for _ in 0..num_annotations {
                        annotations.push(annotation::new(input)?);
                    }
                    parameter_annotations.push(annotation_list { num_annotations, annotations })
                }
                RuntimeInvisibleParameterAnnotations_attribute { num_parameters, parameter_annotations }
            }
            "AnnotationDefault" => {
                let default_value = element_value::new(input)?;
                AnnotationDefault_attribute { default_value }
            }
            "BootstrapMethods" => {
                let num_bootstrap_methods = input.read_u16::<BigEndian>()?;
                let mut bootstrap_methods = Vec::with_capacity(num_bootstrap_methods as usize);
                for _ in 0..num_bootstrap_methods {
                    let bootstrap_method_ref = input.read_u16::<BigEndian>()?;
                    let num_bootstrap_arguments = input.read_u16::<BigEndian>()?;
                    let mut bootstrap_arguments = Vec::with_capacity(num_bootstrap_arguments as usize);
                    for _ in 0..num_bootstrap_arguments {
                        bootstrap_arguments.push(input.read_u16::<BigEndian>()?);
                    }
                    bootstrap_methods.push(bootstrap_method {
                        bootstrap_method_ref,
                        num_bootstrap_arguments,
                        bootstrap_arguments
                    })
                }
                BootstrapMethods_attribute { num_bootstrap_methods, bootstrap_methods }
            }
            _ => {
                println!("Read Unknown Attribute: {}", name);
                let mut infoVec = vec![0u8; attribute_length as usize];
                input.read_exact(&mut infoVec)?;
                Unknown_attribute { info: infoVec }
            }
        })
    }
}

#[derive(Debug)]
struct exception_info {
    start_pc: u16,
    end_pc: u16,
    handler_pc: u16,
    catch_type: u16
}

enum InnerClassAccessFlag {
    ACC_PUBLIC      = 0x0001,
    ACC_PRIVATE     = 0x0002,
    ACC_PROTECTED   = 0x0004,
    ACC_STATIC      = 0x0008,
    ACC_FINAL       = 0x0010,
    ACC_INTERFACE   = 0x0200,
    ACC_ABSTRACT    = 0x0400,
    ACC_SYNTHETIC   = 0x1000,
    ACC_ANNOTATION  = 0x2000,
    ACC_ENUM        = 0x4000,
}
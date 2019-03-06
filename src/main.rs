#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::io;
use class_loader::ClassLoader;
use typed_arena::Arena;
use std::time::{SystemTime};

extern crate byteorder;
extern crate cesu8;
#[macro_use]
extern crate bitflags;
extern crate core;
extern crate typed_arena;
extern crate zip;

mod constant_pool;
mod class_file;
mod attribute;
mod field;
mod method;
mod class_loader;
mod class;
mod class_array;

#[allow(unused_variables)]
fn main() -> io::Result<()>{
    let start = SystemTime::now();
    println!("Hello, world!");
    let mut class_path = Vec::new();
    class_path.push(String::from("/usr/lib/jvm/java-8-oracle/jre/lib/rt.jar"));
    class_path.push(String::from("/home/robot_rover/Desktop/javaTest/"));
    let string_allocator = Arena::new();
    let allocator = Arena::new();
    let mut loader = ClassLoader::new(class_path, &allocator, &string_allocator);
    let class = loader.create_class("Square");
    let main = loader.create_class("Main");
    let interface = loader.create_class("NoOp");
    let array = loader.create_class("[LMain;");
    println!("{:#?}", main);
    let since_start = SystemTime::now().duration_since(start).unwrap();
    println!("Duration: {:?}", since_start);
    Ok(())
}

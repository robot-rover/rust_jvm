use class_file::ClassFile;
use std::collections::HashMap;
use class::Class;
use class::Class::*;
use class_array::ClassArray;
use std::fs::File;
use std::ops::Index;
use class_file::ClassLoadingError::*;
use class_file::ClassLoadingError;
use std::collections::HashSet;
use class::ClassAccessFlag;
use std::path::PathBuf;
use std::path::Path;
use std::cell::RefCell;
use typed_arena::Arena;
use class::ClassRef::{Symbolic, Static};
use constant_pool::cp_info::*;
use std::io::Read;
use zip::ZipArchive;
use zip::read::ZipFile;
use std::io;
use std::fs;
use std::ops::DerefMut;
use std::io::Cursor;
use field;
use class_loader::ClassPath::{Directory, Jar};

pub struct ClassLoader<'a, 'b: 'a> {
    classpath: Vec<ClassPath>,
    class_map: HashMap<String, &'a RefCell<Class<'a>>>,
    strings: &'b Arena<String>,
    classes: &'a Arena<RefCell<Class<'a>>>
}

impl<'b, 'a> ClassLoader<'a, 'b> {
    pub fn new(classpath: Vec<String>, allocator: &'a Arena<RefCell<Class<'a>>>, string_allocator: &'b Arena<String>) -> Self {
        ClassLoader {
            classpath: classpath.iter().map(|s| ClassLoader::to_classpath(s.as_str()).unwrap()).collect(),
            class_map: HashMap::new(),
            strings: string_allocator,
            classes: allocator
        }
    }

    /// Converts a string to the appropriate ClassPath object
    fn to_classpath(path: &str) -> Result<ClassPath, ClassLoadingError> {
        if path.ends_with(".jar") {
            let mut archive_file = File::open(path)?;
            let mut archive = ZipArchive::new(archive_file)?;
            Ok(Jar(archive))
        } else {
            Ok(Directory(path.to_owned()))
        }
    }

    /// Place a loaded class into the list of classes
    fn register_class(&mut self, class_name: &str, class: Class<'a>) -> &'a RefCell<Class<'a>> {
        let class_ref = self.classes.alloc(RefCell::new(class));
        self.class_map.insert(String::from(class_name), class_ref);
        class_ref
    }

    /// Get a reference an existing class or load one
    pub fn create_class(&mut self, class_name: &'a str) -> &'a RefCell<Class<'a>> {
        self.create_class_rec(class_name, &mut HashSet::new())
    }

    //TODO: Don't do 2 lookups if a class is already loaded
    /// create_class but with a Set to prevent cyclic inheritance
    pub fn create_class_rec(&mut self, class_name: &'a str, inheritance_stack: &mut HashSet<String>) -> &'a RefCell<Class<'a>> {
        let already_loaded = self.class_map.contains_key(class_name);
        if !already_loaded {
            let class = self.load_class(class_name, inheritance_stack);
            self.register_class(class_name, class)
        } else {
            self.get_class(class_name)
        }
    }

    /// gets a class from the list of classes, panicking if it doesn't exist
    fn get_class(&self, class_name: &str) -> &'a RefCell<Class<'a>> {
        self.class_map.index(class_name).clone()
    }

    /// Create a new class from a name
    fn load_class(&mut self, class_name: &'a str, inheritance_stack: &mut HashSet<String>) -> Class<'a> {
        let class = if class_name.starts_with('[') {
            Array(self.load_array_class(class_name))
        } else {
            File(self.load_file_class(class_name, inheritance_stack).unwrap())
        };
        println!("Loaded Class: {}", class.get_name());
        class
    }

    /// Create an array class based on a component and a number of diemsions
    ///
    /// The type will be a number of '[' characters followed by a component type
    fn load_array_class(&mut self, class_name: &'a str) -> ClassArray<'a> {
        let mut name_chars = class_name.chars();
        let mut dimensions: u8 = 0;
        while name_chars.next().map_or_else(|| false, |c| c == '[') {
            dimensions += 1;
        }
        let component_type_str = &class_name[(dimensions as usize)..];
        let component_type = field::parse_field_descriptor(&mut component_type_str.chars().enumerate().peekable(), component_type_str);
        ClassArray::new(dimensions, component_type, class_name)
    }

    /// Create a class by attempting to load a .class file from the classpath
    fn load_file_class(&mut self, class_name: &str, inheritance_stack: &mut HashSet<String>) -> Result<ClassFile<'a>, ClassLoadingError> {
        println!("Attempting to load Class: {}", class_name);
        let bytes = self.search_classpath(class_name)?;
        let mut stream = Cursor::new(bytes);
        // Load and parse the the .class file
        let class = ClassFile::new(&mut stream, self.strings)?;

        // If this class has already been loaded
        if self.class_map.contains_key(class.get_name()) {
            return Err(LinkageError)
        }

        // If the class contained doesn't match the filename
        if class_name != class.get_name() {
            return Err(NoClassDefFoundError)
        }

        // The list of classes loaded recursively contains this class
        if inheritance_stack.contains(class_name) {
            return Err(ClassCircularityError)
        }

        {
            if !class.has_super_class() {
                return if class.get_name().eq("java/lang/Object") {
                    Ok(class)
                } else {
                    Err(ClassFormatError(String::from("Class does not have direct superclass")))
                }
            }

            let super_class_ref = class.get_super_class().as_ref().unwrap();

            // Prevent this class from being loaded again, creating infinite recursion
            inheritance_stack.insert(String::from(class.get_name()));

            println!("During {}, recursing to superclass {:?}", class_name, class.get_super_class());

            let super_class_name = match super_class_ref {
                Symbolic(index) => *index,
                Static(class_ref) => panic!("Class is being loaded but already linked: {:#?}", class_ref)
            };

            let super_class = self.create_class_rec(super_class_name, inheritance_stack);

            let is_interface = super_class.borrow().get_access_flags().intersects(ClassAccessFlag::ACC_INTERFACE);

            if is_interface {
                return Err(IncompatibleClassChangeError)
            }
        }

        Ok(class)
    }

    /// Search the classpath for a specific class
    /// eg: java/lang/Object
    fn search_classpath(&mut self, class_name: &str) -> Result<Vec<u8>, ClassLoadingError> {
        let mut class_file_name = String::from(class_name);
        class_file_name.push_str(".class");
        let target_path = Path::new(&class_file_name);
        for classpath_dir in &mut self.classpath {
            match classpath_dir {
                Directory(path) =>
                    if let Some(path) = ClassLoader::search_directory(
                    path.as_str(), class_file_name.as_str())? {
                    return Ok(path)
                },
                Jar(archive) =>
                    if let Some(path) = ClassLoader::search_archive(
                        archive, class_file_name.as_str())? {
                        return Ok(path)
                    }
            }
        }

        // Could not find class anywhere in classpath
        Err(NoClassDefFoundError)
    }

    /// Searches a filesystem folder structure for a named class
    fn search_directory(base_dir: &str, class_file_name: &str) -> Result<Option<Vec<u8>>, ClassLoadingError> {
        let mut path = PathBuf::from(base_dir);
        path.push(class_file_name);
        println!("Loading {:?} from {:?}", class_file_name, path);
        if path.exists() {
            let file = File::open(path)?;
            let bytes = file.bytes().map(|i| i.unwrap()).collect();
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    /// Searches a .jar archive for a named class
    fn search_archive(archive: &mut ZipArchive<File>, class_file_name: &str) -> Result<Option<Vec<u8>>, ClassLoadingError> {
        let mut archive_entry = archive.by_name(class_file_name);
        if let Ok(zip_stream) = archive_entry {
            let bytes = zip_stream.bytes().map(|i| i.unwrap()).collect();
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }
}

enum ClassPath {
    Directory(String),
    Jar(ZipArchive<File>)
}
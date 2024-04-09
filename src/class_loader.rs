use class::Class::*;
use class::{ClassAccessFlag, Class, ClassRef};
use class_array::ClassArray;
use class_file::ClassFile;
use class_file::ClassLoadingError;
use class_file::ClassLoadingError::*;
use field;
use std::cell::{RefCell, Ref};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Cursor;
use std::ops::Index;
use typed_arena::Arena;
use lazy::LazyResolve;
use class_path::{ClassPath, search_classpath};
use class_path::path_to_classpath;
use field::FieldDescriptor::Reference;
use field::FieldDescriptor;

pub struct ClassLoader<'a> {
    classpath: Vec<ClassPath>,
    class_map: HashMap<String, &'a RefCell<Class<'a>>>,
    strings: &'a Arena<String>,
    classes: &'a Arena<RefCell<Class<'a>>>,
}

impl<'a> LazyResolve<'a, RefCell<Class<'a>>> for &'a mut ClassLoader<'a> {
    fn resolve(&mut self, name: &'a str) -> &'a RefCell<Class<'a>> {
        self.create_class(name)
    }
}

impl<'a> ClassLoader<'a> {
    pub fn new(
        classpath: Vec<String>,
        allocator: &'a Arena<RefCell<Class<'a>>>,
        string_allocator: &'a Arena<String>,
    ) -> Self {
        ClassLoader {
            classpath: classpath
                .iter()
                .map(|s| path_to_classpath(s.as_str()).unwrap())
                .collect(),
            class_map: HashMap::new(),
            strings: string_allocator,
            classes: allocator,
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
    fn create_class_rec(
        &mut self,
        class_name: &'a str,
        inheritance_stack: &mut HashSet<String>,
    ) -> &'a RefCell<Class<'a>> {
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
        self.class_map.index(class_name)
    }

    /// Create a new class from a name
    fn load_class(
        &mut self,
        class_name: &'a str,
        inheritance_stack: &mut HashSet<String>,
    ) -> Class<'a> {
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
        let component_type_str: &'a str = &class_name[(dimensions as usize)..];
        let mut component_type: FieldDescriptor<'a> = field::parse_field_descriptor(
            &mut component_type_str.chars().enumerate().peekable(),
            component_type_str,
        );
        if let Reference(class_ref) = &mut component_type {
            let class: &mut ClassRef<'a> = class_ref;
            class.resolve(&mut self);
        }
        ClassArray::new(dimensions, component_type, class_name)
    }

    /// Create a class by attempting to load a .class file from the classpath
    fn load_file_class(
        &mut self,
        class_name: &str,
        inheritance_stack: &mut HashSet<String>,
    ) -> Result<ClassFile<'a>, ClassLoadingError> {
        println!("Attempting to load Class: {}", class_name);
        let bytes = search_classpath(&mut self.classpath, class_name)?;
        let mut stream = Cursor::new(bytes);
        // Load and parse the the .class file
        let mut class = ClassFile::new(&mut stream, self.strings)?;

        // If this class has already been loaded
        if self.class_map.contains_key(class.get_name()) {
            return Err(LinkageError);
        }

        // If the class contained doesn't match the filename
        if class_name != class.get_name() {
            return Err(NoClassDefFoundError);
        }

        // The list of classes loaded recursively contains this class
        if inheritance_stack.contains(class_name) {
            return Err(ClassCircularityError);
        }

        {
            if !class.has_super_class() {
                if !class.get_name().eq("java/lang/Object") {
                    return Err(ClassFormatError(String::from(
                        "Class does not have direct superclass",
                    )));
                }
            } else {
                // Prevent this class from being loaded again, creating infinite recursion
                inheritance_stack.insert(String::from(class.get_name()));


                let super_class_ref = class.resolve_super_class();

                println!(
                    "During {}, recursing to superclass {:?}",
                    class_name,
                    super_class_ref
                );

                let super_class = super_class_ref.as_mut().unwrap().resolve(&mut self);

                let super_is_interface = super_class
                    .borrow()
                    .get_access_flags()
                    .intersects(ClassAccessFlag::ACC_INTERFACE);

                if super_is_interface {
                    return Err(IncompatibleClassChangeError);
                }


            };
        }

        Ok(class)
    }

    fn link_class(&mut self, class: &mut ClassFile<'a>) {}
}

use std::borrow::ToOwned;
use std::fs::File;
use zip::ZipArchive;
use class_path::ClassPath::{Jar, Directory};
use class_file::ClassLoadingError;
use class_file::ClassLoadingError::NoClassDefFoundError;
use std::io::Read;
use std::path::PathBuf;

type ClassPathList = Vec<ClassPath>;

pub enum ClassPath {
    Directory(String),
    Jar(ZipArchive<File>),
}

/// Converts a string to the appropriate ClassPath object
pub fn path_to_classpath(path: &str) -> Result<ClassPath, ClassLoadingError> {
    if path.ends_with(".jar") {
        let archive_file = File::open(path)?;
        let archive = ZipArchive::new(archive_file)?;
        Ok(Jar(archive))
    } else {
        Ok(Directory(path.to_owned()))
    }
}

/// Search the classpath for a specific class
    /// eg: java/lang/Object
pub fn search_classpath(class_path_list: &mut ClassPathList, class_name: &str) -> Result<Vec<u8>, ClassLoadingError> {
    let mut class_file_name = String::from(class_name);
    class_file_name.push_str(".class");
    for classpath_dir in class_path_list {
        match classpath_dir {
            Directory(path) => {
                if let Some(path) =
                search_directory(path.as_str(), class_file_name.as_str())?
                {
                    return Ok(path);
                }
            }
            Jar(archive) => {
                if let Some(path) =
                search_archive(archive, class_file_name.as_str())?
                {
                    return Ok(path);
                }
            }
        }
    }

    // Could not find class anywhere in classpath
    Err(NoClassDefFoundError)
}

/// Searches a filesystem folder structure for a named class
fn search_directory(
    base_dir: &str,
    class_file_name: &str,
) -> Result<Option<Vec<u8>>, ClassLoadingError> {
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
fn search_archive(
    archive: &mut ZipArchive<File>,
    class_file_name: &str,
) -> Result<Option<Vec<u8>>, ClassLoadingError> {
    let archive_entry = archive.by_name(class_file_name);
    if let Ok(zip_stream) = archive_entry {
        let bytes = zip_stream.bytes().map(|i| i.unwrap()).collect();
        Ok(Some(bytes))
    } else {
        Ok(None)
    }
}
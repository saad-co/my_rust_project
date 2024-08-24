use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
enum Permissions {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone)]
enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone)]
enum INode {
    Folder {
        contents: HashMap<String, INode>,
        permissions: Permissions,
    },
    File {
        data: Vec<u8>,
        permissions: Permissions,
    },
}

// Error handling for file system operations
#[derive(Debug)]
enum FileSystemError {
    InvalidType,
    PermissionDenied,
    FileNotFound,
    FileExists,
    DirectoryNotEmpty,
    InvalidFileDescriptor,
}

trait FileSystem {
    fn create(&mut self, path: &str, permissions_mode: Permissions) -> Result<usize, FileSystemError>;
}

struct SimpleFileSystem {
    root: INode,
}

impl FileSystem for SimpleFileSystem {
    fn create(&mut self, path: &str, permissions_mode: Permissions) -> Result<usize, FileSystemError> {
        // This is where we will implement the file creation logic
        // For now, let's print the path and permissions to check the flow
        println!("Creating file at path: {} with permissions: {:?}", path, permissions_mode);

        // Temporary implementation, to be replaced with actual logic
        Ok(1) // Placeholder file descriptor
    }
}

// Function to mount the file system
pub fn mount() -> Box<dyn FileSystem> {
    let root = INode::Folder {
        contents: HashMap::new(),
        permissions: Permissions::ReadWrite,
    };

    Box::new(SimpleFileSystem { root })
}

fn main() {
    let mut fs = mount();
    println!("File system mounted successfully!");

    // Test creating a file
    match fs.create("/new_file.txt", Permissions::ReadWrite) {
        Ok(fd) => println!("File created successfully with file descriptor: {}", fd),
        Err(e) => println!("Error creating file: {:?}", e),
    }
}
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
        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = &mut self.root;

        for (i, component) in components.iter().enumerate() {
            if let INode::Folder { contents, .. } = current {
                if i == components.len() - 1 {
                    // Last component, create the file or directory here
                    if contents.contains_key(*component) {
                        return Err(FileSystemError::FileExists);
                    }
                    contents.insert(component.to_string(), INode::File {
                        data: Vec::new(),
                        permissions: permissions_mode,
                    });
                    return Ok(1); // Placeholder file descriptor
                } else {
                    // Intermediate folder
                    match contents.get_mut(*component) {
                        Some(INode::Folder { .. }) => current = contents.get_mut(*component).unwrap(),
                        _ => return Err(FileSystemError::InvalidType),
                    }
                }
            } else {
                return Err(FileSystemError::InvalidType);
            }
        }

        Err(FileSystemError::InvalidType) // If we get here, path parsing failed
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

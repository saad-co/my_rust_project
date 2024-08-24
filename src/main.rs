use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
enum Permissions {
    Read,
    Write,
    ReadWrite,
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

// File Descriptor Table Entry
#[derive(Debug)]
struct FileDescriptor {
    inode: Arc<Mutex<INode>>,
}

trait FileSystem {
    fn create(
        &mut self,
        path: &str,
        permissions_mode: Permissions,
    ) -> Result<usize, FileSystemError>;

    fn open(&mut self, path: &str) -> Result<usize, FileSystemError>;

    fn close(&mut self, fd: usize) -> Result<(), FileSystemError>;
}

struct SimpleFileSystem {
    root: INode,
    file_descriptors: HashMap<usize, FileDescriptor>,
    next_fd: usize,
}

impl SimpleFileSystem {
    fn new() -> Self {
        let root = INode::Folder {
            contents: HashMap::new(),
            permissions: Permissions::ReadWrite,
        };

        SimpleFileSystem {
            root,
            file_descriptors: HashMap::new(),
            next_fd: 1, // Start file descriptors from 1
        }
    }

    fn allocate_fd(&mut self, inode: Arc<Mutex<INode>>) -> usize {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.file_descriptors.insert(fd, FileDescriptor { inode });
        fd
    }

    fn get_inode(&self, path: &str) -> Result<Arc<Mutex<INode>>, FileSystemError> {
        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = &self.root;

        for component in components.iter() {
            let component_str = component.to_string();
            if let INode::Folder { contents, .. } = current {
                match contents.get(&component_str) {
                    Some(node) => current = node,
                    None => return Err(FileSystemError::FileNotFound),
                }
            } else {
                return Err(FileSystemError::InvalidType);
            }
        }

        match current {
            INode::File { .. } => Ok(Arc::new(Mutex::new(current.clone()))),
            _ => Err(FileSystemError::InvalidType),
        }
    }
}

impl FileSystem for SimpleFileSystem {
    fn create(
        &mut self,
        path: &str,
        permissions_mode: Permissions,
    ) -> Result<usize, FileSystemError> {
        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = &mut self.root;

        for (i, component) in components.iter().enumerate() {
            let component_str = component.to_string();
            if let INode::Folder { contents, .. } = current {
                if i == components.len() - 1 {
                    if contents.contains_key(&component_str) {
                        return Err(FileSystemError::FileExists);
                    }
                    contents.insert(
                        component_str.clone(),
                        INode::File {
                            data: Vec::new(),
                            permissions: permissions_mode,
                        },
                    );
                    let inode = Arc::new(Mutex::new(contents.get(&component_str).unwrap().clone()));
                    return Ok(self.allocate_fd(inode));
                } else {
                    match contents.get_mut(&component_str) {
                        Some(INode::Folder { .. }) => {
                            current = contents.get_mut(&component_str).unwrap()
                        }
                        _ => return Err(FileSystemError::InvalidType),
                    }
                }
            } else {
                return Err(FileSystemError::InvalidType);
            }
        }

        Err(FileSystemError::InvalidType)
    }

    fn open(&mut self, path: &str) -> Result<usize, FileSystemError> {
        let inode = self.get_inode(path)?;
        Ok(self.allocate_fd(inode))
    }

    fn close(&mut self, fd: usize) -> Result<(), FileSystemError> {
        if self.file_descriptors.remove(&fd).is_some() {
            Ok(())
        } else {
            Err(FileSystemError::InvalidFileDescriptor)
        }
    }
}

// Function to mount the file system
pub fn mount() -> Box<dyn FileSystem> {
    Box::new(SimpleFileSystem::new())
}

fn main() {
    let mut fs = mount();
    println!("File system mounted successfully!");

    // Test creating a file
    match fs.create("/new_file.txt", Permissions::ReadWrite) {
        Ok(fd) => {
            println!("File created successfully with file descriptor: {}", fd);

            // Test opening the file
            match fs.open("/new_file.txt") {
                Ok(open_fd) => {
                    println!("File opened successfully with file descriptor: {}", open_fd);

                    // Test closing the file
                    match fs.close(open_fd) {
                        Ok(()) => println!("File closed successfully."),
                        Err(e) => println!("Error closing file: {:?}", e),
                    }
                }
                Err(e) => println!("Error opening file: {:?}", e),
            }
        }
        Err(e) => println!("Error creating file: {:?}", e),
    }
}

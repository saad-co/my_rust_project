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

#[derive(Debug, Clone, Copy)]
enum OffsetFrom {
    Start(usize),
    Current(isize),
    End(isize),
}

// File Descriptor Table Entry
#[derive(Debug)]
struct FileDescriptor {
    inode: Arc<Mutex<INode>>,
    position: usize,
}

trait FileSystem {
    fn create(
        &mut self,
        path: &str,
        permissions_mode: Permissions,
    ) -> Result<usize, FileSystemError>;

    fn open(&mut self, path: &str) -> Result<usize, FileSystemError>;

    fn close(&mut self, fd: usize) -> Result<(), FileSystemError>;

    fn write(&mut self, fd: usize, data: &[u8]) -> Result<(), FileSystemError>;
    fn read(&self, fd: usize, buffer: &mut [u8]) -> Result<usize, FileSystemError>;
    fn seek(&mut self, fd: usize, offset: OffsetFrom) -> Result<usize, FileSystemError>;
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

    fn get_file_descriptor(&self, fd: usize) -> Result<Arc<Mutex<INode>>, FileSystemError> {
        self.file_descriptors
            .get(&fd)
            .map(|desc| desc.inode.clone())
            .ok_or(FileSystemError::InvalidFileDescriptor)
    }

    fn allocate_fd(&mut self, inode: Arc<Mutex<INode>>) -> usize {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.file_descriptors.insert(fd, FileDescriptor { inode, position: 0 });
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

    fn write(&mut self, fd: usize, data: &[u8]) -> Result<(), FileSystemError> {
        let mut file_desc = self.file_descriptors
            .get_mut(&fd)
            .ok_or(FileSystemError::InvalidFileDescriptor)?;

        let mut inode = file_desc.inode.lock().unwrap();
        if let INode::File { data: file_data, .. } = &mut *inode {
            file_data.extend_from_slice(data);
            Ok(())
        } else {
            Err(FileSystemError::InvalidType)
        }
    }

    fn read(&self, fd: usize, buffer: &mut [u8]) -> Result<usize, FileSystemError> {
        let file_desc = self.file_descriptors
            .get(&fd)
            .ok_or(FileSystemError::InvalidFileDescriptor)?;

        let mut inode = file_desc.inode.lock().unwrap();
        if let INode::File { data: file_data, .. } = &*inode {
            let start = file_desc.position;
            let end = start + buffer.len();
            let len = end.min(file_data.len()) - start;
            buffer[..len].copy_from_slice(&file_data[start..start + len]);
            Ok(len)
        } else {
            Err(FileSystemError::InvalidType)
        }
    }

    fn seek(&mut self, fd: usize, offset: OffsetFrom) -> Result<usize, FileSystemError> {
        let mut file_desc = self
            .file_descriptors
            .get_mut(&fd)
            .ok_or(FileSystemError::InvalidFileDescriptor)?;

        let inode = file_desc.inode.lock().unwrap();
        let file_size = if let INode::File { data, .. } = &*inode {
            data.len()
        } else {
            return Err(FileSystemError::InvalidType);
        };

        let new_position = match offset {
            OffsetFrom::Start(pos) => pos,
            OffsetFrom::Current(offset) => {
                if let Some(pos) = file_desc.position.checked_add_signed(offset) {
                    pos
                } else {
                    return Err(FileSystemError::InvalidType);
                }
            }
            OffsetFrom::End(offset) => {
                if let Some(pos) = file_size.checked_add_signed(offset) {
                    pos
                } else {
                    return Err(FileSystemError::InvalidType);
                }
            }
        };

        file_desc.position = new_position.min(file_size);
        Ok(file_desc.position)
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

            // Test writing to the file
            match fs.write(fd, b"Hello, world!") {
                Ok(()) => println!("Data written successfully."),
                Err(e) => println!("Error writing data: {:?}", e),
            }

            // Test seeking to the start of the file
            match fs.seek(fd, OffsetFrom::Start(0)) {
                Ok(pos) => println!("Seeked to position: {}", pos),
                Err(e) => println!("Error seeking file: {:?}", e),
            }

            // Test reading from the file
            let mut buffer = [0; 13];
            match fs.read(fd, &mut buffer) {
                Ok(bytes_read) => {
                    println!(
                        "Read {} bytes: {:?}",
                        bytes_read,
                        String::from_utf8_lossy(&buffer)
                    );
                }
                Err(e) => println!("Error reading data: {:?}", e),
            }

            // Test seeking to the end of the file
            match fs.seek(fd, OffsetFrom::End(-6)) {
                Ok(pos) => println!("Seeked to position: {}", pos),
                Err(e) => println!("Error seeking file: {:?}", e),
            }

            // Test reading from the new position
            let mut buffer = [0; 6];
            match fs.read(fd, &mut buffer) {
                Ok(bytes_read) => {
                    println!(
                        "Read {} bytes: {:?}",
                        bytes_read,
                        String::from_utf8_lossy(&buffer)
                    );
                }
                Err(e) => println!("Error reading data: {:?}", e),
            }

            // Test closing the file
            match fs.close(fd) {
                Ok(()) => println!("File closed successfully."),
                Err(e) => println!("Error closing file: {:?}", e),
            }
        }
        Err(e) => println!("Error creating file: {:?}", e),
    }
}
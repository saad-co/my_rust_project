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
    fn mkdir(&mut self, path: &str) -> Result<(), FileSystemError>;
    fn rmdir(&mut self, path: &str) -> Result<(), FileSystemError>;
    // fn unlink(&mut self, path: &str) -> Result<(), FileSystemError>;
    // fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), FileSystemError>;
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
            next_fd: 1,
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
        self.file_descriptors
            .insert(fd, FileDescriptor { inode, position: 0 });
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
                    current = contents
                        .get_mut(&component_str)
                        .ok_or(FileSystemError::FileNotFound)?;
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
        self.file_descriptors
            .remove(&fd)
            .map(|_| ())
            .ok_or(FileSystemError::InvalidFileDescriptor)
    }

    fn write(&mut self, fd: usize, data: &[u8]) -> Result<(), FileSystemError> {
        let inode = self.get_file_descriptor(fd)?;

        let mut node = inode.lock().unwrap();
        match &mut *node {
            INode::File {
                data: file_data, ..
            } => {
                file_data.extend_from_slice(data);
                Ok(())
            }
            _ => Err(FileSystemError::InvalidType),
        }
    }

    fn read(&self, fd: usize, buffer: &mut [u8]) -> Result<usize, FileSystemError> {
        let inode = self.get_file_descriptor(fd)?;

        let node = inode.lock().unwrap();
        match &*node {
            INode::File {
                data: file_data, ..
            } => {
                let read_len = buffer.len().min(file_data.len());
                buffer[..read_len].copy_from_slice(&file_data[..read_len]);
                Ok(read_len)
            }
            _ => Err(FileSystemError::InvalidType),
        }
    }

    fn seek(&mut self, fd: usize, offset: OffsetFrom) -> Result<usize, FileSystemError> {
        let descriptor = self
            .file_descriptors
            .get_mut(&fd)
            .ok_or(FileSystemError::InvalidFileDescriptor)?;

        let inode = descriptor.inode.lock().unwrap();
        match &*inode {
            INode::File { data, .. } => {
                let new_position = match offset {
                    OffsetFrom::Start(pos) => pos,
                    OffsetFrom::Current(off) => (descriptor.position as isize + off) as usize,
                    OffsetFrom::End(off) => (data.len() as isize + off) as usize,
                };
                if new_position > data.len() {
                    return Err(FileSystemError::InvalidType);
                }
                descriptor.position = new_position;
                Ok(descriptor.position)
            }
            _ => Err(FileSystemError::InvalidType),
        }
    }

    fn mkdir(&mut self, path: &str) -> Result<(), FileSystemError> {
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
                        component_str,
                        INode::Folder {
                            contents: HashMap::new(),
                            permissions: Permissions::ReadWrite,
                        },
                    );
                    return Ok(());
                } else {
                    current = contents
                        .get_mut(&component_str)
                        .ok_or(FileSystemError::FileNotFound)?;
                }
            } else {
                return Err(FileSystemError::InvalidType);
            }
        }

        Err(FileSystemError::InvalidType)
    }

    fn rmdir(&mut self, path: &str) -> Result<(), FileSystemError> {
        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = &mut self.root;

        if components.is_empty() {
            return Err(FileSystemError::FileNotFound);
        }

        for (i, component) in components.iter().enumerate() {
            let component_str = component.to_string();

            if let INode::Folder { contents, .. } = current {
                if i == components.len() - 1 {
                    // We are at the target directory
                    if let Some(node) = contents.get(&component_str) {
                        match node {
                            INode::Folder {
                                contents: folder_contents,
                                ..
                            } => {
                                if folder_contents.is_empty() {
                                    contents.remove(&component_str);
                                    return Ok(());
                                } else {
                                    return Err(FileSystemError::DirectoryNotEmpty);
                                }
                            }
                            _ => return Err(FileSystemError::InvalidType),
                        }
                    } else {
                        return Err(FileSystemError::FileNotFound);
                    }
                } else {
                    match contents.get_mut(&component_str) {
                        Some(INode::Folder { .. }) => {
                            current = contents.get_mut(&component_str).unwrap();
                        }
                        _ => return Err(FileSystemError::InvalidType),
                    }
                }
            } else {
                return Err(FileSystemError::InvalidType);
            }
        }

        Err(FileSystemError::FileNotFound)
    }

    // fn unlink(&mut self, path: &str) -> Result<(), FileSystemError> {
    //     let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    //     let mut current = &mut self.root;
    //     let mut parent: Option<&mut HashMap<String, INode>> = None;
    //     let mut component_name = String::new();

    //     for (i, component) in components.iter().enumerate() {
    //         let component_str = component.to_string();

    //         if let INode::Folder { contents, .. } = current {
    //             if i == components.len() - 1 {
    //                 // We are at the target file
    //                 if let Some(node) = contents.get(&component_str) {
    //                     match node {
    //                         INode::File { .. } => {
    //                             component_name = component_str;
    //                             parent = Some(contents);
    //                         }
    //                         _ => return Err(FileSystemError::InvalidType),
    //                     }
    //                 } else {
    //                     return Err(FileSystemError::FileNotFound);
    //                 }
    //             } else {
    //                 match contents.get_mut(&component_str) {
    //                     Some(INode::Folder { .. }) => {
    //                         current = contents.get_mut(&component_str).unwrap();
    //                     }
    //                     _ => return Err(FileSystemError::InvalidType),
    //                 }
    //             }
    //         } else {
    //             return Err(FileSystemError::InvalidType);
    //         }
    //     }

    //     // If we found the parent and the file to delete, perform the deletion
    //     if let Some(contents) = parent {
    //         contents.remove(&component_name);
    //         Ok(())
    //     } else {
    //         Err(FileSystemError::FileNotFound)
    //     }
    // }

    //     fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), FileSystemError> {
    //         let old_components: Vec<&str> = old_path.trim_start_matches('/').split('/').collect();
    //         let new_components: Vec<&str> = new_path.trim_start_matches('/').split('/').collect();

    //         if old_components.is_empty() || new_components.is_empty() {
    //             return Err(FileSystemError::InvalidType);
    //         }

    //         let mut current = &mut self.root;
    //         let mut parent: Option<&mut HashMap<String, INode>> = None;
    //         let mut component_name = String::new();

    //         for (i, component) in old_components.iter().enumerate() {
    //             let component_str = component.to_string();

    //             if let INode::Folder { contents, .. } = current {
    //                 if i == old_components.len() - 1 {
    //                     // We are at the target file or folder
    //                     if contents.contains_key(&component_str) {
    //                         component_name = component_str;
    //                         parent = Some(contents);
    //                     } else {
    //                         return Err(FileSystemError::FileNotFound);
    //                     }
    //                 } else {
    //                     match contents.get_mut(&component_str) {
    //                         Some(INode::Folder { .. }) => {
    //                             current = contents.get_mut(&component_str).unwrap();
    //                         }
    //                         _ => return Err(FileSystemError::InvalidType),
    //                     }
    //                 }
    //             } else {
    //                 return Err(FileSystemError::InvalidType);
    //             }
    //         }

    //         // Move the file/folder to the new location
    //         if let Some(contents) = parent {
    //             let inode = contents.remove(&component_name).unwrap();
    //             let mut new_current = &mut self.root;

    //             for (i, component) in new_components.iter().enumerate() {
    //                 let component_str = component.to_string();

    //                 if let INode::Folder { contents, .. } = new_current {
    //                     if i == new_components.len() - 1 {
    //                         if contents.contains_key(&component_str) {
    //                             return Err(FileSystemError::FileExists);
    //                         } else {
    //                             contents.insert(component_str, inode);
    //                             return Ok(());
    //                         }
    //                     } else {
    //                         match contents.get_mut(&component_str) {
    //                             Some(INode::Folder { .. }) => {
    //                                 new_current = contents.get_mut(&component_str).unwrap();
    //                             }
    //                             _ => return Err(FileSystemError::InvalidType),
    //                         }
    //                     }
    //                 } else {
    //                     return Err(FileSystemError::InvalidType);
    //                 }
    //             }
    //         }
    //         Err(FileSystemError::InvalidType)
    //     }
}

fn mount() -> SimpleFileSystem {
    SimpleFileSystem::new()
}

fn main() {
    let mut fs = mount();

    // Create a new directory
    match fs.mkdir("/my_folder") {
        Ok(_) => println!("Directory '/my_folder' created successfully."),
        Err(e) => println!("Failed to create directory: {:?}", e),
    }

    // List the root directory (should contain "my_folder")
    match fs.mkdir("/") {
        Ok(_) => println!("Root directory listed successfully."),
        Err(e) => println!("Failed to list root directory: {:?}", e),
    }

    // Create a new file in the directory
    match fs.create("/my_folder/my_file.txt", Permissions::ReadWrite) {
        Ok(fd) => {
            println!("File '/my_folder/my_file.txt' created successfully with fd: {}", fd);

            // Write data to the file
            let data = b"Hello, World!";
            match fs.write(fd, data) {
                Ok(_) => println!("Data written to '/my_folder/my_file.txt' successfully."),
                Err(e) => println!("Failed to write data: {:?}", e),
            }

            // Seek to the beginning of the file
            match fs.seek(fd, OffsetFrom::Start(0)) {
                Ok(pos) => println!("Seek successful. New position: {}", pos),
                Err(e) => println!("Failed to seek: {:?}", e),
            }

            // Read data from the file
            let mut buffer = vec![0; data.len()];
            match fs.read(fd, &mut buffer) {
                Ok(bytes_read) => println!(
                    "Data read from '/my_folder/my_file.txt': {}",
                    String::from_utf8_lossy(&buffer[..bytes_read])
                ),
                Err(e) => println!("Failed to read data: {:?}", e),
            }

            // Close the file
            match fs.close(fd) {
                Ok(_) => println!("File '/my_folder/my_file.txt' closed successfully."),
                Err(e) => println!("Failed to close file: {:?}", e),
            }
        }
        Err(e) => println!("Failed to create file: {:?}", e),
    }

    // Remove the empty directory
    match fs.rmdir("/my_folder") {
        Ok(_) => println!("Directory '/my_folder' removed successfully."),
        Err(e) => println!("Failed to remove directory: {:?}", e),
    }
}

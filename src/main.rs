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
    fn mkdir(&mut self, path: &str) -> Result<(), FileSystemError>;
    fn rmdir(&mut self, path: &str) -> Result<(), FileSystemError>;
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

        Ok(Arc::new(Mutex::new(current.clone())))
    }

    fn mkdir(&mut self, path: &str) -> Result<(), FileSystemError> {
        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = &mut self.root;

        for (i, component) in components.iter().enumerate() {
            let component_str = component.to_string();
            if let INode::Folder {
                contents,
                permissions,
            } = current
            {
                if i == components.len() - 1 {
                    // We are at the last component
                    if contents.contains_key(&component_str) {
                        return Err(FileSystemError::FileExists);
                    }
                    if *permissions != Permissions::Write && *permissions != Permissions::ReadWrite
                    {
                        return Err(FileSystemError::PermissionDenied);
                    }
                    contents.insert(
                        component_str.clone(),
                        INode::Folder {
                            contents: HashMap::new(),
                            permissions: Permissions::ReadWrite,
                        },
                    );
                    return Ok(());
                } else {
                    match contents.get_mut(&component_str) {
                        Some(INode::Folder { .. }) => {
                            current = contents.get_mut(&component_str).unwrap()
                        }
                        Some(INode::File { .. }) => return Err(FileSystemError::InvalidType),
                        None => return Err(FileSystemError::FileNotFound),
                    }
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
        let mut file_desc = self
            .file_descriptors
            .get_mut(&fd)
            .ok_or(FileSystemError::InvalidFileDescriptor)?;

        let mut inode = file_desc.inode.lock().unwrap();
        if let INode::File {
            data: file_data, ..
        } = &mut *inode
        {
            file_data.extend_from_slice(data);
            Ok(())
        } else {
            Err(FileSystemError::InvalidType)
        }
    }

    fn read(&self, fd: usize, buffer: &mut [u8]) -> Result<usize, FileSystemError> {
        let file_desc = self
            .file_descriptors
            .get(&fd)
            .ok_or(FileSystemError::InvalidFileDescriptor)?;

        let inode = file_desc.inode.lock().unwrap();
        if let INode::File { data, .. } = &*inode {
            let len = data.len();
            buffer.copy_from_slice(&data[0..len]);
            Ok(len)
        } else {
            Err(FileSystemError::InvalidType)
        }
    }

    fn seek(&mut self, fd: usize, offset: OffsetFrom) -> Result<usize, FileSystemError> {
        let file_desc = self
            .file_descriptors
            .get_mut(&fd)
            .ok_or(FileSystemError::InvalidFileDescriptor)?;

        let inode = file_desc.inode.lock().unwrap();
        if let INode::File { data, .. } = &*inode {
            let new_pos = match offset {
                OffsetFrom::Start(pos) => pos,
                OffsetFrom::Current(delta) => {
                    if delta < 0 {
                        (file_desc.position as isize + delta) as usize
                    } else {
                        (file_desc.position as isize + delta) as usize
                    }
                }
                OffsetFrom::End(delta) => {
                    if delta < 0 {
                        (data.len() as isize + delta) as usize
                    } else {
                        (data.len() as isize + delta) as usize
                    }
                }
            };

            if new_pos > data.len() {
                return Err(FileSystemError::InvalidFileDescriptor);
            }

            file_desc.position = new_pos;
            Ok(new_pos)
        } else {
            Err(FileSystemError::InvalidType)
        }
    }

    fn mkdir(&mut self, path: &str) -> Result<(), FileSystemError> {
        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current = &mut self.root;

        for (i, component) in components.iter().enumerate() {
            let component_str = component.to_string();
            if let INode::Folder {
                contents,
                permissions,
            } = current
            {
                if i == components.len() - 1 {
                    // We are at the last component
                    if contents.contains_key(&component_str) {
                        return Err(FileSystemError::FileExists);
                    }
                    if *permissions != Permissions::Write && *permissions != Permissions::ReadWrite
                    {
                        return Err(FileSystemError::PermissionDenied);
                    }
                    contents.insert(
                        component_str.clone(),
                        INode::Folder {
                            contents: HashMap::new(),
                            permissions: Permissions::ReadWrite,
                        },
                    );
                    return Ok(());
                } else {
                    match contents.get_mut(&component_str) {
                        Some(INode::Folder { .. }) => {
                            current = contents.get_mut(&component_str).unwrap()
                        }
                        Some(INode::File { .. }) => return Err(FileSystemError::InvalidType),
                        None => return Err(FileSystemError::FileNotFound),
                    }
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
}

// Function to mount the file system
pub fn mount() -> Box<dyn FileSystem> {
    Box::new(SimpleFileSystem::new())
}

fn main() {
    let mut fs = mount();
    println!("File system mounted successfully!");

    // Test creating a directory
    match fs.mkdir("/dir_to_remove") {
        Ok(()) => println!("Directory created successfully."),
        Err(e) => println!("Error creating directory: {:?}", e),
    }

    // Test removing an empty directory
    match fs.rmdir("/dir_to_remove") {
        Ok(()) => println!("Directory removed successfully."),
        Err(e) => println!("Error removing directory: {:?}", e),
    }

    // Test removing a non-existing directory
    match fs.rmdir("/non_existent_dir") {
        Ok(()) => println!("Non-existent directory removed successfully (unexpected)."),
        Err(e) => println!("Error removing non-existent directory: {:?}", e),
    }

    // Recreate the directory and add a nested directory
    match fs.mkdir("/dir_to_remove") {
        Ok(()) => println!("Directory created successfully."),
        Err(e) => println!("Error creating directory: {:?}", e),
    }

    match fs.mkdir("/dir_to_remove/nested_dir") {
        Ok(()) => println!("Nested directory created successfully."),
        Err(e) => println!("Error creating nested directory: {:?}", e),
    }

    // Test removing a non-empty directory
    match fs.rmdir("/dir_to_remove") {
        Ok(()) => println!("Non-empty directory removed successfully (unexpected)."),
        Err(e) => println!("Error removing non-empty directory: {:?}", e),
    }

    // Test removing the nested directory first, then the parent
    match fs.rmdir("/dir_to_remove/nested_dir") {
        Ok(()) => println!("Nested directory removed successfully."),
        Err(e) => println!("Error removing nested directory: {:?}", e),
    }

    match fs.rmdir("/dir_to_remove") {
        Ok(()) => println!("Parent directory removed successfully."),
        Err(e) => println!("Error removing parent directory: {:?}", e),
    }
}

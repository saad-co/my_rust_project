Simplified File System in Rust

This project is a simplified implementation of a file system in Rust, created to demonstrate how basic file system operations can be implemented using Rust's standard library. The file system supports creating, reading, writing, and managing files and directories with an emphasis on permissions and error handling.

Project Structure
File System Structure: The file system is represented as a tree, where each node (INode) can either be a File or a Folder. The tree starts with a root folder that contains other files or folders.

INode: This enum defines the type of file system node. It can either be a File or a Folder. Each INode has permissions associated with it (Read, Write, ReadWrite).

FileDescriptor: This struct represents an open file with a specific position in the file data.

FileSystem Trait: Defines the core operations that the file system must support, including creating files, opening them, reading, writing, seeking, and directory management.

SimpleFileSystem: The main struct implementing the FileSystem trait. It maintains the root directory, file descriptors, and methods to manipulate the file system.

Key Design Decisions
1. INode Representation
The file system is modeled as a tree of INodes, where each node can either be a File or a Folder. This approach simplifies the structure and management of files and directories, allowing for straightforward traversal and manipulation.

2. Permissions Management
Each INode has associated permissions (Read, Write, ReadWrite) that govern how the file or folder can be accessed. Permissions are checked before performing operations like reading or writing, ensuring proper access control.

3. File Descriptors
File descriptors are used to represent open files. Each file descriptor is associated with an INode and maintains a position within the file for reading or writing. This allows for efficient and stateful file operations.

4. Error Handling
Comprehensive error handling is implemented to manage invalid operations, such as accessing non-existent files, violating permissions, or attempting to read beyond the end of a file. Errors are returned as Result types, allowing for clean and safe error management.

5. Multi-Threaded Safety
The implementation uses Arc<Mutex<INode>> to ensure that file system operations are safe in a multi-threaded environment. This design choice allows multiple threads to access and modify the file system concurrently without race conditions.

Usage
Mounting the File System
let mut fs = mount();
Creating Files
let fd = fs.create("/myfile.txt", Permissions::ReadWrite).unwrap();
Writing to Files
fs.write(fd, b"Hello, World!").unwrap();
Reading from Files
let mut buffer = vec![0; 13];
fs.read(fd, &mut buffer).unwrap();
println!("{}", String::from_utf8(buffer).unwrap()); // Prints "Hello, World!"
Seeking in Files
fs.seek(fd, OffsetFrom::Start(0)).unwrap();
Creating Directories
fs.mkdir("/myfolder").unwrap();
Removing Directories
fs.rmdir("/myfolder").unwrap();

Limitations
The current implementation does not support renaming or deleting individual files (unlink).
Conclusion
This project showcases how basic file system operations can be implemented in Rust, with a focus on safety, concurrency, and error handling. The design choices, such as using a tree structure for INodes and implementing multi-threaded safety, make the file system both efficient and robust.
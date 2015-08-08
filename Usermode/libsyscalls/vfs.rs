//
//
//
pub struct Node(super::ObjectHandle);
pub struct File(super::ObjectHandle, u64);
pub struct Dir(super::ObjectHandle);
pub struct Symlink(super::ObjectHandle);

pub use ::values::VFSError as Error;
pub use ::values::VFSNodeType as NodeType;

#[repr(C,u32)]
pub enum FileOpenMode
{
	None	 = 0,
	ReadOnly = 1,
	Execute  = 2,
	// TODO: Write modes
}
#[repr(C,u8)]
pub enum MemoryMapMode
{
	/// Read-only mapping of a file
	ReadOnly = 0,
	/// Executable mapping of a file
	Execute = 1,
	/// Copy-on-write (used for executable files)
	COW = 2,
	/// Allows writing to the backing file
	WriteBack = 3,
}

fn to_obj(val: usize) -> Result<super::ObjectHandle, Error> {
	super::ObjectHandle::new(val).map_err(|code| Error::from(code))
}
fn to_result(val: usize) -> Result<u32, Error> {
	super::to_result(val).map_err(|code| Error::from(code))
}

impl Node
{
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Node, Error> {
		let path = path.as_ref();
		// SAFE: Syscall
		to_obj( unsafe { syscall!(VFS_OPENNODE, path.as_ptr() as usize, path.len()) } as usize )
			.map(|h| Node(h))
	}

	pub fn class(&self) -> NodeType {
		// SAFE: Syscall with no side-effects
		NodeType::from( unsafe { self.0.call_0(::values::VFS_NODE_GETTYPE) } as u32 )
	}

	pub fn into_dir(self) -> Result<Dir,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_0(::values::VFS_NODE_TODIR) } as usize )
			.map(|h| Dir(h))
	}
	pub fn into_file(self, mode: FileOpenMode) -> Result<File,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_0(::values::VFS_NODE_TOFILE) } as usize )
			.map(|h| File(h, 0))
	}
	pub fn into_symlink(self) -> Result<Symlink,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_0(::values::VFS_NODE_TOLINK) } as usize )
			.map(|h| Symlink(h))
	}
}
impl ::Object for Node {
	const CLASS: u16 = ::values::CLASS_VFS_NODE;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Node(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn get_wait(&self) -> ::values::WaitItem {
		self.0.get_wait(0)
	}
	fn check_wait(&self, _wi: &::values::WaitItem) {
	}
}

impl File
{
	pub fn open<T: AsRef<[u8]>>(path: T, mode: FileOpenMode) -> Result<File,Error> {
		let path = path.as_ref();
		// SAFE: Syscall
		to_obj( unsafe { syscall!(VFS_OPENFILE, path.as_ptr() as usize, path.len(), mode as u32 as usize) } as usize )
			.map(|h| File(h, 0))
	} 
	
	pub fn get_size(&self) -> u64 { panic!("TODO: File::get_size") }
	pub fn get_cursor(&self) -> u64 { self.1 }
	pub fn set_cursor(&mut self, pos: u64) { self.1 = pos; }
	
	pub fn read(&mut self, data: &mut [u8]) -> Result<usize,Error> {
		let count = try!( self.read_at(self.1, data) );
		self.1 += count as u64;
		Ok(count)
	}
	pub fn read_at(&self, ofs: u64, data: &mut [u8]) -> Result<usize,Error> {
		assert!(::core::mem::size_of::<usize>() == ::core::mem::size_of::<u64>());
		// SAFE: Passes valid arguments to READAT
		unsafe {
			match ::to_result( self.0.call_3(::values::VFS_FILE_READAT, ofs as usize, data.as_ptr() as usize, data.len()) as usize )
			{
			Ok(v) => Ok(v as usize),
			Err(v) => {
				panic!("TODO: Error code {}", v);
				}
			}
		}
	}
	
	pub fn write_at(&self, ofs: u64, data: &[u8]) -> Result<usize,Error> {
		assert!(::core::mem::size_of::<usize>() == ::core::mem::size_of::<u64>());
		// SAFE: All validated
		unsafe {
			match ::to_result( self.0.call_3(
				::values::VFS_FILE_WRITEAT,
				ofs as usize, data.as_ptr() as usize, data.len()
				) as usize )
			{
			Ok(v) => Ok(v as usize),
			Err(v) => {
				panic!("TODO: Error code {}", v);
				}
			}
		}
	}
	
	// Actualy safe, as it uses the aliasing restrictions from the file, and checks memory ownership
	pub fn memory_map(&self, ofs: u64, read_size: usize, mem_addr: usize, mode: MemoryMapMode) -> Result<(),Error> {
		assert!(::core::mem::size_of::<usize>() == ::core::mem::size_of::<u64>());
		// SAFE: Passes valid arguments to MEMMAP
		unsafe {
			match ::to_result( self.0.call_4(::values::VFS_FILE_MEMMAP, ofs as usize, read_size, mem_addr, mode as u8 as usize) as usize )
			{
			Ok(_) => Ok( () ),
			Err(v) => {
				panic!("TODO: Error code {}", v);
				}
			}
		}
	}
}
impl ::Object for File {
	const CLASS: u16 = ::values::CLASS_VFS_FILE;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		File(handle, 0)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn get_wait(&self) -> ::values::WaitItem {
		self.0.get_wait(0)
	}
	fn check_wait(&self, _wi: &::values::WaitItem) {
	}
}


impl ::std_io::Read for File {
	fn read(&mut self, buf: &mut [u8]) -> ::std_io::Result<usize> {
		match self.read(buf)
		{
		Ok(v) => Ok(v),
		Err(v) => {
			panic!("VFS File read err: {:?}", v);
			},
		}
	}
}
impl ::std_io::Seek for File {
	fn seek(&mut self, pos: ::std_io::SeekFrom) -> ::std_io::Result<u64> {
		use std_io::SeekFrom;
		match pos
		{
		SeekFrom::Start(pos) => self.set_cursor(pos),
		SeekFrom::End(ofs) => {
			let pos = if ofs < 0 {
				self.get_size() - (-ofs) as u64
				} else {
				self.get_size() + ofs as u64
				};
			self.set_cursor(pos);
			},
		SeekFrom::Current(ofs) => {
			let pos = if ofs < 0 {
				self.get_cursor() - (-ofs) as u64
				} else {
				self.get_cursor() + ofs as u64
				};
			self.set_cursor(pos);
			},
		}
		Ok(self.get_cursor())
	}
}

impl Dir
{
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Dir, Error> {
		let path = path.as_ref();
		// SAFE: Syscall
		match super::ObjectHandle::new( unsafe { syscall!(VFS_OPENDIR, path.as_ptr() as usize, path.len()) } as usize )
		{
		Ok(rv) => Ok( Dir(rv) ),
		Err(code) => Err( From::from(code) ),
		}
	}

	pub fn read_ent<'a>(&mut self, namebuf: &'a mut [u8]) -> Result<&'a [u8], Error> {
		// SAFE: Syscall
		let len = try!(to_result(unsafe { self.0.call_2(::values::VFS_DIR_READENT, namebuf.as_ptr() as usize, namebuf.len()) } as usize ));
		Ok( &namebuf[ .. len as usize] )
	}
}
impl ::Object for Dir {
	const CLASS: u16 = ::values::CLASS_VFS_DIR;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Dir(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn get_wait(&self) -> ::values::WaitItem {
		self.0.get_wait(0)
	}
	fn check_wait(&self, _wi: &::values::WaitItem) {
	}
}


impl Symlink
{
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Symlink, Error> {
		let path = path.as_ref();
		// SAFE: Syscall
		to_obj( unsafe { syscall!(VFS_OPENLINK, path.as_ptr() as usize, path.len()) } as usize )
			.map(|h| Symlink(h))
	}

	pub fn read_target<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], Error> {
		// SAFE: Syscall with correct args
		let len = try!(to_result( unsafe { self.0.call_2(::values::VFS_LINK_READ, buf.as_mut_ptr() as usize, buf.len()) } as usize ));
		Ok( &buf[ .. len as usize] )
	}
}
impl ::Object for Symlink {
	const CLASS: u16 = ::values::CLASS_VFS_LINK;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Symlink(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn get_wait(&self) -> ::values::WaitItem {
		self.0.get_wait(0)
	}
	fn check_wait(&self, _wi: &::values::WaitItem) {
	}
}

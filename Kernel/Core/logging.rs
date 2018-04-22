// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/boot.rs
///! Kernel logging framework
///!
///! All kernel logging goes through this module, using the `log_*` macros, each corresponding
///! to a one of the logging levels in `Levels`.
// TODO: Support registerable log sinks, where all >Debug logs go (why not all?)
// > Cache/buffer sink
// > Display sink
// > Serial sink
#[allow(unused_imports)]
use prelude::*;
use core::fmt;
use arch::sync::Spinlock;

/// Log level, ranging from a kernel panic down to tracing
#[repr(u16)]
#[derive(PartialEq,PartialOrd,Copy,Clone)]
pub enum Level
{
	/// Everything broke
	LevelPanic = 0,
	/// Something broke
	LevelError = 1,
	/// Recoverable
	LevelWarning = 2,
 	/// Odd
	LevelNotice = 3,
   	/// Interesting (least important for the user)
	LevelInfo = 4,
	/// General (highest developer-only level)
	LevelLog = 5,
   	/// What
	LevelDebug = 6,
  	/// Where
	LevelTrace = 7,
}

#[doc(hidden)]
pub enum Colour
{
	Default,
	Red,
	Yellow,
	Green,
	Blue,
	Purple,
	Grey,
}

#[doc(hidden)]
pub struct LoggingFormatter<'a>
{
	lock_handle: ::arch::sync::HeldSpinlock<'a,Sinks>,
	
	// NOTE: Must be second, forcing interrupts to be reenabled after the lock is released
	_irq_handle: ::arch::sync::HeldInterrupts,
}

/// Wrapper around a &-ptr that prints a hexdump of the passed data.
pub struct HexDump<'a,T: ?Sized + 'a>(pub &'a T);

/// Wrapper around a `&[u8]` to print it as an escaped byte string
pub struct RawString<'a>(pub &'a [u8]);

static S_LOGGING_LOCK: Spinlock<Sinks> = Spinlock::new( Sinks { serial: serial::Sink, memory: None, video: None } );

trait Sink
{
	/// Start a new log entry
	fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str);
	/// Append data to the current log entry
	fn write(&mut self, data: &str);
	/// End a log entry
	fn end(&mut self);
}
struct Sinks
{
	serial: serial::Sink,
	memory: Option<memory::Sink>,
	video: Option<video::Sink>,
}

mod serial
{
	#[allow(unused_imports)]
	use prelude::*;
	use super::{Level,Colour};
	use core::fmt;
	
	pub struct Sink;
	impl super::Sink for Sink
	{
		fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str) {
			use core::fmt::Write;
			self.set_colour(level.to_colour());
			write!(self, "{:6}{} {}[{}] - ", timestamp, level, ::threads::get_thread_id(), source).unwrap();
		}
		fn write(&mut self, s: &str) {
			::arch::puts(s);
		}
		fn end(&mut self) {
			::arch::puts("\x1b[0m\n");
		}
	}
	impl fmt::Write for Sink
	{
		fn write_str(&mut self, s: &str) -> fmt::Result {
			::arch::puts(s);
			Ok( () )
		}
	}
	impl Sink
	{
		/// Set the output colour of the formatter
		pub fn set_colour(&self, colour: Colour) {
			match colour
			{
			Colour::Default => ::arch::puts("\x1b[0m"),
			Colour::Red	 => ::arch::puts("\x1b[31m"),
			Colour::Green   => ::arch::puts("\x1b[32m"),
			Colour::Yellow  => ::arch::puts("\x1b[33m"),
			Colour::Blue	=> ::arch::puts("\x1b[34m"),
			Colour::Purple  => ::arch::puts("\x1b[35m"),
			Colour::Grey	=> ::arch::puts("\x1b[1;30m"),
			}
		}
	}
}

mod memory
{
	#[allow(unused_imports)]
	use prelude::*;
	use super::Level;
	
	pub struct Sink
	{
		lines: ::lib::ring_buffer::RingBuf<LogMessage>,
	}
	// Buffer for log data
	// Temp hack until type-level ints are avaliable
	struct LogDataBuf([u8;160]);
	impl LogDataBuf {
		fn new() -> LogDataBuf {
			// SAFE: Plain old data
			LogDataBuf(unsafe{::core::mem::zeroed()})
		}
	}
	impl ::core::convert::AsRef<[u8]> for LogDataBuf {
		fn as_ref(&self) -> &[u8] { &self.0 }
	}
	impl ::core::convert::AsMut<[u8]> for LogDataBuf {
		fn as_mut(&mut self) -> &mut [u8] { &mut self.0 }
	}
	#[allow(dead_code)]	// Allow unread fields
	struct LogMessage
	{
		time: ::time::TickCount,
		level: Level,
		source: &'static str,
		data: ::lib::string::FixedString<LogDataBuf>,
	}
	impl Sink
	{
		pub fn new() -> Sink {
			Sink {
				lines: ::lib::ring_buffer::RingBuf::new(256),	// 256 log of scrollback
			}
		}
	}
	impl super::Sink for Sink
	{
		fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str) {
			let new_line = LogMessage {
				time: timestamp, level: level, source: source,
				data: ::lib::string::FixedString::new(LogDataBuf::new())
				};
			match self.lines.push_back( new_line )
			{
			Ok(_) => {},
			Err(_new_line) => {
				todo!("Handle rollover of kernel log");
				},
			}
		}
		fn write(&mut self, s: &str) {
			self.lines.back_mut().unwrap().data.push_str(s);
		}
		fn end(&mut self) {
			// No action required
		}
	}
}

mod video
{
	//use prelude::*;
	use super::Level;
	
	pub struct Sink;
	
	impl super::Sink for Sink
	{
		fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str) {
			// Acquire a writer from the GUI
			// - TODO: requires acquiring the lock on the kernel log, which is a Mutex, and may already be held.
			// - What about having the kernel log write methods be unsafe, then they can assume that they're called in logging
			//   context (which is unique already)
			// Write header
			todo!("VideoSink - {} {} {}", timestamp, level, source);
		}
		fn write(&mut self, s: &str) {
			// Pass through
			todo!("video::Sink::write - '{}'", s);
		}
		fn end(&mut self) {
			// Drop writer (replace with None)
			unimplemented!();
		}
	}
}

impl Level
{
	fn to_flag(&self) -> char
	{
		match *self
		{
		Level::LevelPanic   => 'k',
		Level::LevelError   => 'e',
		Level::LevelWarning => 'w',
		Level::LevelNotice  => 'n',
		Level::LevelInfo	=> 'i',
		Level::LevelLog	 => 'l',
		Level::LevelDebug   => 'd',
		Level::LevelTrace   => 't',
		}
	}
	fn to_colour(&self) -> Colour
	{
		match *self
		{
		Level::LevelPanic   => Colour::Purple,
		Level::LevelError   => Colour::Red,
		Level::LevelWarning => Colour::Yellow,
		Level::LevelNotice  => Colour::Green,
		Level::LevelLog	 => Colour::Blue,
		Level::LevelTrace   => Colour::Grey,
		_ => Colour::Default,
		}
	}
}

impl fmt::Display for Level
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.to_flag())
	}
}

impl Sinks
{
	pub fn foreach_mut<Fcn: FnMut(&mut Sink)>(&mut self, mut f: Fcn)
	{
		f(&mut self.serial);
		self.memory.as_mut().map(|x| f(x));
		self.video .as_mut().map(|x| f(x));
	}
}

impl<'a> LoggingFormatter<'a>
{
	/// Create a new logging formatter
	#[is_safe(irq)]	// SAFE: This lock holds interrupts, so can't interrupt itself.
	pub fn new(level: Level, modname: &'static str) -> LoggingFormatter<'static>
	{
		// TODO: if S_LOGGING_LOCK is held by the current CPU, error.
		let mut rv = LoggingFormatter {
				_irq_handle: ::arch::sync::hold_interrupts(),
				lock_handle: S_LOGGING_LOCK.lock()
			};
		let ts = ::time::ticks();
		rv.lock_handle.foreach_mut(|x| x.start(ts, level, modname));
		rv
	}
}

impl<'a> fmt::Write for LoggingFormatter<'a>
{
	fn write_str(&mut self, s: &str) -> fmt::Result
	{
		self.lock_handle.foreach_mut(|x| x.write(s));
		Ok( () )
	}
}
impl<'a> ::core::ops::Drop for LoggingFormatter<'a>
{
	fn drop(&mut self)
	{
		self.lock_handle.foreach_mut(|x| x.end());
	}
}

impl<'a, T: ?Sized + 'a> HexDump<'a,T>
{
	/// Return the wrapped type as a &[u8]
	fn byteslice(&self) -> &[u8]
	{
		let size = ::core::mem::size_of_val::<T>(&self.0);
		// SAFE: Memory is valid, and cast is allowed
		unsafe { ::core::slice::from_raw_parts( self.0 as *const T as *const u8, size ) }
	}
}

impl<'a, T: ?Sized + 'a> fmt::Debug for HexDump<'a,T>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		let slice = self.byteslice();
		try!(write!(f, "{} bytes: ", slice.len()));
		for (idx,v) in slice.iter().enumerate()
		{
			try!(write!(f, "{:02x} ", *v));
			if idx % 16 == 15 {
				try!(write!(f, "| "));
			}
		}
		Ok( () )
	}
}

impl<'a> fmt::Debug for RawString<'a>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		try!(write!(f, "b\""));
		for &b in self.0 {
			match b
			{
			b'\t' => try!(write!(f, "\\t")),
			b'\n' => try!(write!(f, "\\n")),
			b'\\' => try!(write!(f, "\\\\")),
			b'"' => try!(write!(f, "\\\"")),
			32 ... 0x7E => try!(write!(f, "{}", b as char)),
			_ => try!(write!(f, "\\x{:02x}", b)),
			}
		}
		try!(write!(f, "\""));
		Ok( () )
	}
}

struct HexDumpBlk<'a>(&'a [u8]);
impl<'a> fmt::Display for HexDumpBlk<'a>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		assert!(self.0.len() <= 16);
		for i in 0 .. 16
		{
			if i == 8 {
				try!(write!(f, " "));
			}
			if i < self.0.len() {
				try!(write!(f, "{:02x} ", self.0[i]));
			}
			else {
				try!(write!(f, "   "));
			}
			
		}
		try!(write!(f, "|"));
		for i in 0 .. 16
		{
			if i < self.0.len() {
				try!(write!(f, "{}",
					match self.0[i]
					{
					v @ 32 ... 0x7E => v as char,
					_ => '.',
					}));
			}
			else {
				try!(write!(f, " "));
			}
		}
		try!(write!(f, "|"));
		Ok( () )
	}
}
pub fn hex_dump_t<T>(label: &str, data: &T) {
	// SAFE: Casting to an immutable byte representation is valid
	let slice = unsafe { ::core::slice::from_raw_parts(data as *const T as *const u8, ::core::mem::size_of::<T>()) };
	hex_dump(label, slice);
}
pub fn hex_dump(label: &str, data: &[u8])
{
	log_debug!("{} Dump {:p}+{}", label, &data[0], data.len());
	for block in data.chunks(16)
	{
		log_debug!("{} {:p}: {}", label, &block[0], HexDumpBlk(block));
	}
}

pub fn print_iter<I: Iterator>(i: I) -> PrintIter<I> { PrintIter(::core::cell::RefCell::new(Some(i))) }
pub struct PrintIter<I: Iterator>(::core::cell::RefCell<Option<I>>);
macro_rules! print_iter_def {
	($($t:ident),+) => {$(
		impl<I: Iterator> ::core::fmt::$t for PrintIter<I>
		where
			<I as Iterator>::Item: ::core::fmt::$t
		{
			fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				let mut i = self.0.borrow_mut().take().unwrap();
				try!(write!(f, "["));
				if let Some(v) = i.next()
				{
					try!(v.fmt(f));
					for v in i {
						try!(write!(f, ","));
						try!(v.fmt(f));
					}
				}
				try!(write!(f, "]"));
				Ok( () )
			}
		})*
	};
}
print_iter_def! { LowerHex }

pub fn start_memory_sink() {
	let sink = memory::Sink::new();
	
	let _irq = ::arch::sync::hold_interrupts();
	let mut lh = S_LOGGING_LOCK.lock();
	
	if lh.memory.is_none()
	{
		lh.memory = Some( sink );
	}
}

#[doc(hidden)]
/// Returns true if the passed combination of module and level is enabled
pub fn enabled(level: Level, modname: &str) -> bool
{
	if modname == "kernel::unwind" {
		return true;
	}

	#[repr(C)]
	struct LogCfgEnt {
		name_ptr: *const u8,
		name_len: u16,
		level: u16,
		#[cfg(target_pointer_width="64")]
		_pad: u32,
	}
	extern "C" {
		static log_cfg: [LogCfgEnt; 0];
		static log_cfg_end: [LogCfgEnt; 0];
	}
	// SAFE: This data doesn't change (only does pointers)
	let log_ent_count = (unsafe { log_cfg_end.as_ptr() as usize - log_cfg.as_ptr() as usize }) / ::core::mem::size_of::<LogCfgEnt>();
	// SAFE: Assembly defines these symbols, and I hope it gets the format right
	let log_ents = unsafe { ::core::slice::from_raw_parts(log_cfg.as_ptr(), log_ent_count) };
	for ent in log_ents {
		// SAFE: They're UTF-8 strings from assembly.
		let ent_modname = unsafe { ::core::str::from_utf8_unchecked( ::core::slice::from_raw_parts(ent.name_ptr, ent.name_len as usize) ) };
		if modname == ent_modname {
			return (level as u16) < ent.level;
		}
	}

	true
}

#[doc(hidden)]
/// Returns a logging formatter
pub fn getstream(level: Level, modname: &'static str) -> LoggingFormatter
{
	assert!( enabled(level, modname) );
	LoggingFormatter::new(level, modname)
}


// vim: ft=rust


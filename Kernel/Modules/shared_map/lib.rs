// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/shared_map/lib.rs
//! A key-value map that internally handles synchronisation
//!
//! A wrapper around RwLock<VecMap>
#![no_std]
#![feature(const_fn)]
use kernel::sync::rwlock::{RwLock, self};

extern crate kernel;

pub struct SharedMap<K: Send+Sync+Ord,V: Send+Sync>
{
	lock: RwLock<SharedMapInner<K,V,>>,
}
struct SharedMapInner<K: Send+Sync+Ord, V: Send+Sync>
{
	m: ::kernel::lib::collections::VecMap<K,V>,
}

impl<K: Send+Sync+Ord, V: Send+Sync> SharedMap<K,V>
{
	pub const fn new() -> Self {
		SharedMap {
			lock: RwLock::new(SharedMapInner { m: ::kernel::lib::collections::VecMap::new_const() }),
			}
	}
	pub fn get(&self, k: &K) -> Option<Handle<K,V>> {
		let lh = self.lock.read();
		let p = lh.m.get(k).map(|r| r as *const _);
		// SAFE: Lock handle is carried with the pointer, pointer can't be invalidated until that handle is dropped
		p.map(|ptr| unsafe { Handle {
			_ref_handle: lh,
			data_ptr: &*ptr,
			}})
	}
	pub fn take(&self, k: &K) -> Option<V> {
		let mut lh = self.lock.write();
		lh.m.remove(k)
	}
	pub fn insert(&self, k: K, v: V) {
		let mut lh = self.lock.write();
		lh.m.insert(k, v);
	}
}
pub struct Handle<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync>
{
	_ref_handle: rwlock::Read<'a, SharedMapInner<K,V>>,
	data_ptr: &'a V,
}
impl<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync> ::core::ops::Deref for Handle<'a, K, V>
{
	type Target = V;
	fn deref(&self) -> &V {
		self.data_ptr
	}
}


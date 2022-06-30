use std::io::Error;
use std::ops::{Deref, DerefMut};
use std::ptr::null_mut;
use std::sync::Arc;

use libc::c_void;

lazy_static::lazy_static! {
    pub static ref PAGESIZE:usize=unsafe{libc::sysconf(libc::_SC_PAGESIZE)}.try_into().expect("Invalid Pagesize");
}

struct Slice {
    ptr: *mut c_void,
    len: usize,
}

unsafe fn alloc_slice(pages: usize) -> Slice {
    let len = pages * (*PAGESIZE);
    //ensures page alignment
    let ptr = libc::mmap(
        null_mut(),
        len,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
        -1,
        0,
    );
    if ptr == libc::MAP_FAILED {
        panic!("Unable to map pages: {}", Error::last_os_error())
    }
    //only ram pages are merged
    if -1 == libc::mlock(ptr, len) {
        panic!(
            "Unable to set pages to lock pages: {}",
            Error::last_os_error()
        )
    }
    //allow merging
    if -1 == libc::madvise(ptr, len, libc::MADV_MERGEABLE) {
        panic!(
            "Unable to set pages to mergeable: {}",
            Error::last_os_error()
        )
    }
    //hugepage merging interferes
    if -1 == libc::madvise(ptr, len, libc::MADV_NOHUGEPAGE) {
        panic!(
            "Unable to prevent hugepege combining: {}",
            Error::last_os_error()
        )
    }
    Slice { ptr, len }
}

impl Drop for Slice {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.ptr, self.len) };
    }
}

pub struct Page {
    ptr: *mut u8,
    _alloc: Arc<Slice>,
}

impl Deref for Page {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, *PAGESIZE) }
    }
}

impl DerefMut for Page {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, *PAGESIZE) }
    }
}

pub fn alloc_single_page() -> Page {
    let slice = unsafe { alloc_slice(1) };
    Page {
        ptr: slice.ptr.cast(),
        _alloc: Arc::from(slice),
    }
}

pub fn alloc_pages(num: usize) -> Vec<Page> {
    let slice = Arc::from(unsafe { alloc_slice(num) });
    (0_usize..num)
        .into_iter()
        .map(|index| Page {
            ptr: unsafe { slice.ptr.add(index * (*PAGESIZE)) }.cast(),
            _alloc: slice.clone(),
        })
        .collect()
}

pub fn fill_page(page: &mut Page) {
    let fill = 0x4f77472055775520_u64;
    let s = unsafe { std::slice::from_raw_parts_mut(page.ptr.cast::<u64>(), (*PAGESIZE) >> 3) };
    s.fill(fill);
}

//! Lab experiment (feature `lab-thread-alloc`): a per-thread cache in front of the system
//! allocator. wasm32's allocator takes one global lock, so allocation-heavy parallel code (the
//! quotient polynomial loop) stops scaling with threads. Small blocks are kept in thread-local
//! free lists by power-of-two size class; everything else goes to the system allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;

const MIN_SHIFT: u32 = 4;
const MAX_SHIFT: u32 = 20;
const CLASSES: usize = (MAX_SHIFT - MIN_SHIFT + 1) as usize;
const MAX_ALIGN: usize = 16;
/// Bytes one thread may keep per size class.
const CACHED_BYTES_PER_CLASS: usize = 2 << 20;

struct Cache {
    heads: [*mut u8; CLASSES],
    counts: [u32; CLASSES],
}

thread_local! {
    static CACHE: UnsafeCell<Cache> = const { UnsafeCell::new(Cache { heads: [std::ptr::null_mut(); CLASSES], counts: [0; CLASSES] }) };
}

fn class_of(layout: &Layout) -> Option<usize> {
    if layout.align() > MAX_ALIGN || layout.size() > (1 << MAX_SHIFT) {
        return None;
    }
    let shift = layout.size().max(1 << MIN_SHIFT).next_power_of_two().trailing_zeros();
    Some((shift - MIN_SHIFT) as usize)
}

fn class_layout(class: usize) -> Layout {
    unsafe { Layout::from_size_align_unchecked(1 << (class as u32 + MIN_SHIFT), MAX_ALIGN) }
}

pub struct ThreadCacheAlloc;

unsafe impl GlobalAlloc for ThreadCacheAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(class) = class_of(&layout) else { return System.alloc(layout) };
        let cached = CACHE
            .try_with(|cache| {
                let cache = &mut *cache.get();
                let head = cache.heads[class];
                if !head.is_null() {
                    cache.heads[class] = *(head as *mut *mut u8);
                    cache.counts[class] -= 1;
                }
                head
            })
            .unwrap_or(std::ptr::null_mut());
        if cached.is_null() {
            System.alloc(class_layout(class))
        } else {
            cached
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let Some(class) = class_of(&layout) else { return System.dealloc(ptr, layout) };
        let limit = (CACHED_BYTES_PER_CLASS >> (class as u32 + MIN_SHIFT)).max(2) as u32;
        let kept = CACHE
            .try_with(|cache| {
                let cache = &mut *cache.get();
                if cache.counts[class] >= limit {
                    return false;
                }
                *(ptr as *mut *mut u8) = cache.heads[class];
                cache.heads[class] = ptr;
                cache.counts[class] += 1;
                true
            })
            .unwrap_or(false);
        if !kept {
            System.dealloc(ptr, class_layout(class));
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        match (class_of(&layout), class_of(&new_layout)) {
            (Some(old), Some(new)) if old == new => ptr,
            (None, None) => System.realloc(ptr, layout, new_size),
            _ => {
                let new_ptr = self.alloc(new_layout);
                if !new_ptr.is_null() {
                    std::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
                    self.dealloc(ptr, layout);
                }
                new_ptr
            }
        }
    }
}

#[global_allocator]
static GLOBAL: ThreadCacheAlloc = ThreadCacheAlloc;

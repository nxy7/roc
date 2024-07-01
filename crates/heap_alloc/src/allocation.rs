/// OS-level virtual memory allocation and deallocation functions, for use in the `arena` module.
/// The goal here is to avoid managing free lists and instead to directly ask the OS for memory.
/// In long-running compiler processes (e.g. watch mode, editor integrations, repl), this can
/// prevent memory usage from slowly growing over time because we're actually goving memory
/// back to the OS when we're done with it.
///
/// Since we should only use these to allocate memory for an entire module at a time, this should
/// result in 1 total syscall per module, which should be fine in terms of performance.
///
/// As of this writing, wasm uses the wee_alloc crate to emulate virtual memory by managing a free
/// list behind the scenes, since wasm only supports growing the heap and that's it. Although
/// wasm doesn't have a watch mode, it does have long-running processes in the form of the repl
/// and also potentially in the future a playground.
use core::{
    alloc::Layout,
    fmt,
    mem::{self, MaybeUninit},
    ptr::NonNull,
};

#[derive(Debug)]
pub struct Allocation {
    pages: NonNull<Page>,
    layout: Layout,
    bytes_used: usize,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct Page {
    _bytes: [u8; PAGE_SIZE],
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Page")
    }
}

// All allocations will be rounded up to this number of bytes.
//
// https://devblogs.microsoft.com/oldnewthing/20210510-00/?p=105200
// 16KiB should be accepted by all Windows systems
#[cfg(any(windows, unix))]
const PAGE_SIZE: usize = 16384;

// All allocations will be rounded up to this number of bytes.
//
// In wasm, "each page is sized 64KiB" according to
// https://developer.mozilla.org/en-US/docs/webassembly/reference/memory/size
#[cfg(target_arch = "wasm32")]
const PAGE_SIZE: usize = 65536;

/// We use wee_alloc for allocations on wasm because wasm natively supports only growing the heap,
/// not releasing anything. Releasing has to be built in userspace, which wee_alloc provides.
#[cfg(target_arch = "wasm32")]
static WEE_ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Debug, PartialEq, Eq)]
pub enum AllocFailed {
    OsAllocFailed,
    InvalidLayout,
}

impl Allocation {
    /// This may round the requested number of bytes up to the nearest page size,
    /// depending on target OS.
    pub fn alloc_virtual(layout: Layout) -> Result<Self, AllocFailed> {
        // Round up to nearest OS page size or the requested alignment,
        // whichevever is bigger. Pad the size to fit this alignment.
        let layout = match layout.align_to(layout.align().max(PAGE_SIZE)) {
            Ok(layout) => layout.pad_to_align(),
            Err(_) => {
                return Err(AllocFailed::InvalidLayout);
            }
        };

        let non_null = {
            #[cfg(unix)]
            {
                use core::{ffi::c_void, ptr};

                extern "C" {
                    fn mmap(
                        addr: *mut c_void,
                        length: usize,
                        prot: i32,
                        flags: i32,
                        fd: i32,
                        offset: i64,
                    ) -> *mut c_void;
                }

                const MAP_FAILED: *mut c_void = -1isize as *mut c_void;
                const PROT_READ: i32 = 1;
                const PROT_WRITE: i32 = 2;
                const MAP_PRIVATE: i32 = 0x0002;

                #[cfg(target_os = "macos")]
                const MAP_ANONYMOUS: i32 = 0x1000;

                #[cfg(target_os = "linux")]
                const MAP_ANONYMOUS: i32 = 0x0020;

                // Safety: We rounded up `size` to the correct multiple already.
                let answer = unsafe {
                    mmap(
                        ptr::null_mut(),
                        layout.size(),
                        PROT_READ | PROT_WRITE,
                        MAP_PRIVATE | MAP_ANONYMOUS,
                        -1,
                        0,
                    )
                };

                match NonNull::new(answer) {
                    Some(non_null) if answer != MAP_FAILED => non_null,
                    _ => {
                        return Err(AllocFailed::OsAllocFailed);
                    }
                }
            }

            #[cfg(windows)]
            {
                use core::{ffi::c_void, ptr};

                extern "system" {
                    fn VirtualAlloc(
                        lpAddress: *mut c_void,
                        dwSize: usize,
                        flAllocationType: u32,
                        flProtect: u32,
                    ) -> *mut c_void;
                }

                const MEM_COMMIT: u32 = 0x1000;
                const MEM_RESERVE: u32 = 0x2000;;
                const PAGE_READWRITE: u32 = 0x04;

                // Safety: We rounded up `size` to the correct multiple already.
                let ptr = unsafe {
                    VirtualAlloc(
                        ptr::null_mut(),
                        layout.size(),
                        MEM_COMMIT | MEM_RESERVE,
                        PAGE_READWRITE,
                    )
                };

                match NonNull::new(ptr) {
                    Some(non_null) => non_null,
                    None => {
                        return Err(AllocFailed::OsAllocFailed);
                    }
                }
            }

            #[cfg(target_arch = "wasm32")]
            {
                let ptr = unsafe { WEE_ALLOC.alloc(layout) };

                // We should never return a size smaller than what was requested!
                debug_assert!(size >= layout.size());

                match NonNull::new(ptr) {
                    Some(non_null) => non_null,
                    None => {
                        return Err(AllocFailed::OsAllocFailed);
                    }
                }
            }
        };

        Ok(Self {
            pages: non_null.cast(),
            bytes_used: 0,
            layout,
        })
    }

    /// Reallocate in-place if possible; otherwise, create a new allocation
    /// and copy over the contents of the old one. If the new size would
    /// exceed isize::MAX, it instead becomes isize::MAX. No matter what,
    /// the new allocation will be
    pub fn grow(&mut self, additional_bytes_desired: usize) -> Result<(), AllocFailed> {
        let layout = self.layout;
        let new_size = layout.size().saturating_add(additional_bytes_desired);

        match Layout::from_size_align(new_size, layout.align()) {
            Ok(layout) => {
                let todo = todo!(); // TODO try to grow the allocation in-place. Replace self's pointer.
            }
            Err(_) => Err(AllocFailed::InvalidLayout),
        }
    }

    #[cfg(any(unix, windows))]
    pub fn read_file_into<'a, T>(
        &'a mut self,
        file: &mut impl ReadFile,
    ) -> Result<&'a mut [T], FromFileErr> {
        // Calculate how many of our bytes will have been used after aligning our allocation
        // as needed to have a slice of these elems.
        let align_offset = self.pages.as_ptr().align_offset(mem::align_of::<T>());
        let aligned_used = self.bytes_used.saturating_add(align_offset);

        // Make sure we actually have space left after aligning
        if self.layout.size() < aligned_used {
            return Err(FromFileErr::NotEnoughSpace);
        }

        // The slice should begin `aligned_used` bytes after our pointer,
        // to ensure that its elements are properly aligned.
        let ptr = (self.pages.as_ptr() as *mut u8).byte_add(aligned_used);

        // The pointer should be correctly aligned for the slice.
        debug_assert_eq!(ptr.align_offset(mem::align_of::<T>()), 0);

        let max_len = self.layout.size().saturating_sub(aligned_used);
        let mut buf = unsafe { core::slice::from_raw_parts_mut(ptr, max_len) };

        match file.read_into(&mut buf) {
            Ok(bytes_read) => {
                if bytes_read < max_len {
                    // Update how many bytes in this allocation are used based on how many we *actually* read.
                    self.bytes_used = aligned_used + bytes_read;

                    Ok(unsafe { core::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len()) })
                } else {
                    Err(FromFileErr::NotEnoughSpace)
                }
            }
            Err(io_err) => FromFileErr::IoErr(io_err),
        }
    }

    /// This does *not* update bytes_used
    unsafe fn mut_slice<T>(&mut self) -> &mut [MaybeUninit<T>] {
        // Calculate how many of our bytes will have been used after aligning our allocation
        // as needed to have a slice of these elems.
        let align_offset = self.pages.as_ptr().align_offset(mem::align_of::<T>());
        let aligned_used = self.bytes_used.saturating_add(align_offset);

        // Make sure we actually have space left after aligning
        if self.layout.size() < aligned_used {
            return None;
        }

        // The slice should begin `aligned_used` bytes after our pointer,
        // to ensure that its elements are properly aligned.
        let ptr = (self.pages.as_ptr() as *mut u8).byte_add(aligned_used);

        // The pointer should be correctly aligned for the slice.
        debug_assert_eq!(ptr.align_offset(mem::align_of::<T>()), 0);

        Some(unsafe { core::slice::from_raw_parts_mut(ptr.cast(), elems) })
    }

    /// Hands out a slice into the allocation. Retrurns None if there is not enough room remaining
    /// in the allocated bytes to fit the requested number of elements.
    pub fn extract_slice<T>(&mut self, elems: usize) -> Option<&mut [MaybeUninit<T>]> {
        // Calculate how many of our bytes will have been used after aligning our allocation
        // as needed to have a slice of these elems.
        let align_offset = self.pages.as_ptr().align_offset(mem::align_of::<T>());
        let aligned_used = self.bytes_used.saturating_add(align_offset);

        // Make sure we can actually fit all the elems (after aligning).
        {
            let bytes_remaining = self.layout.size().saturating_sub(aligned_used);
            let bytes_desired = mem::size_of::<T>() * elems;

            if bytes_desired > bytes_remaining {
                return None;
            }
        }

        // The slice should begin `aligned_used` bytes after our pointer,
        // to ensure that its elements are properly aligned.
        let ptr = (self.pages.as_ptr() as *mut u8).byte_add(aligned_used);

        // The pointer should be correctly aligned for the slice.
        debug_assert_eq!(ptr.align_offset(mem::align_of::<T>()), 0);

        self.bytes_used = aligned_used + (elems * mem::size_of::<T>());

        // We should not have used more than the number of bytes we allocated.
        debug_assert!(self.bytes_used <= self.layout.size());

        Some(unsafe { core::slice::from_raw_parts_mut(ptr.cast(), elems) })
    }

    pub fn bytes_remaining(&self) -> u64 {
        self.layout.size().saturating_sub(self.bytes_used)
    }

    pub fn slice_mut(&mut self, layout: Layout) -> &mut [u8] {
        // We won't return a slice that's bigger than the number of
        // allocated bytes we have left!
        let available_len = self.bytes_remaining();
        let desired_len = available_len.min(layout.size());
        let desired_align = self.layout.align().max(layout.align());

        // Figure out how much padding we need to achieve the desired alignment
        let ptr = self.pages.as_ptr() as *mut u8;
        let padding_needed = ptr.align_offset(desired_align);

        // Figure out what the actual length of the slice will be,
        // taking into account necessary padding and how mny bytes are left.
        let actual_len = desired_len
            .saturating_add(padding_needed)
            .min(available_len);

        // Advance the pointer past the padding.
        let ptr = unsafe { ptr.add(padding_needed) };

        // After adding the padding, the pointer should now be aligned correctly!
        debug_assert_eq!(ptr as usize % desired_align, 0);

        // Record the new length
        self.len += actual_len;

        unsafe { core::slice::from_raw_parts_mut(ptr, actual_len) }
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        let ptr = self.pages.as_ptr();
        let layout = self.layout;
        let size = layout.size();

        #[cfg(unix)]
        {
            use core::ffi::c_void;

            extern "C" {
                fn munmap(addr: *mut c_void, length: usize) -> i32;
            }

            // If deallocation fails, panic in debug builds so we can try to diagnose it
            // (and so that it will fail tests), but silently continue in release builds
            // because a memory leak is generally a better user experience than a crash.
            let _answer = unsafe { munmap(ptr as *mut c_void, size) };

            #[cfg(debug_assertions)]
            {
                if _answer < 0 {
                    panic!("Tried to deallocate address {:?} but it failed!", ptr);
                }
            }
        }

        #[cfg(windows)]
        {
            use core::ffi::c_void;

            extern "system" {
                fn VirtualFree(lpAddress: *mut c_void, dwSize: usize, dwFreeType: u32) -> i32;
            }

            const MEM_RELEASE: u32 = 0x8000;

            // When calling VirtualAlloc with MEM_RELEASE, the second argument must be 0.
            // https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualfree#parameters
            let _answer = unsafe { VirtualFree(ptr as *mut c_void, 0, MEM_RELEASE) };

            #[cfg(debug_assertions)]
            {
                if _answer == 0 {
                    panic!("Tried to deallocate address {:?} but it failed!", ptr);
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ptr = unsafe { WEE_ALLOC.dealloc(layout) };

            // If deallocation fails, panic in debug builds so we can try to diagnose it
            // (and so that it will fail tests), but silently continue in release builds
            // because a memory leak is generally a better user experience than a crash.
            #[cfg(debug_assertions)]
            {
                if _ptr.is_null() {
                    panic!("Tried to deallocate address {:?} but it failed!", ptr);
                }
            }
        }
    }
}

#[test]
fn verify_page_size() {
    let os_page_size = unsafe {
        #[cfg(unix)]
        {
            extern "C" {
                fn getpagesize() -> i32;
            }

            getpagesize() as usize
        }

        #[cfg(windows)]
        {
            // https://devblogs.microsoft.com/oldnewthing/20210510-00/?p=105200
            // 16KiB should be accepted by all Windows systems
            16384
        }

        #[cfg(target_arch = "wasm32")]
        {
            // In wasm, "each page is sized 64KiB" according to
            // https://developer.mozilla.org/en-US/docs/webassembly/reference/memory/size
            65536
        }
    };

    assert_eq!(os_page_size, PAGE_SIZE);
}

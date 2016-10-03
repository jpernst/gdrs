#![feature(allocator)]
#![allocator]
#![no_std]

use core::ptr;

extern crate libc;



extern "C" {
	#[no_mangle]
	fn godot_alloc(p_bytes: libc::c_int) -> *mut libc::c_void;
	#[no_mangle]
	fn godot_realloc(p_ptr: *mut libc::c_void, p_bytes: libc::c_int) -> *mut libc::c_void;
	#[no_mangle]
	fn godot_free(p_ptr: *mut libc::c_void);
}



#[no_mangle]
pub extern fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
	match unsafe { godot_alloc(size as libc::c_int) } as *mut u8 {
		ptr if ptr == ptr::null_mut() => ptr::null_mut(),
		ptr if ptr as usize % align != 0 => {
			unsafe { godot_free(ptr as *mut libc::c_void); }
			ptr::null_mut()
		},
		ptr => ptr,
	}
}



#[no_mangle]
pub extern fn __rust_deallocate(ptr: *mut u8, _old_size: usize, _align: usize) {
	unsafe { godot_free(ptr as *mut libc::c_void) }
}



#[no_mangle]
pub extern fn __rust_reallocate(ptr: *mut u8, _old_size: usize, size: usize, _align: usize) -> *mut u8 {
	unsafe { godot_realloc(ptr as *mut libc::c_void, size as libc::c_int) as *mut u8 }
}



#[no_mangle]
pub extern fn __rust_reallocate_inplace(_ptr: *mut u8, old_size: usize, _size: usize, _align: usize) -> usize {
	old_size
}



#[no_mangle]
pub extern fn __rust_usable_size(size: usize, _align: usize) -> usize {
	size
}

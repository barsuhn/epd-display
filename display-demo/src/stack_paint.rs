use defmt::info;
use core::ptr;
use core::arch::asm;

const STACK_PAINT_VALUE: u32 = 0xDEADBEEF;
const SAFETY_MARGIN_WORDS: usize = 64;

unsafe extern "C" {
    static mut _stack_start: u32;
    static mut _stack_end: u32;
}

#[inline(always)]
fn get_stack_pointer() -> *const u32 {
    let sp: u32;
    unsafe {
        asm!("mov {}, sp", out(reg) sp, options(nomem, nostack, preserves_flags));
    }
    sp as *const u32
}

#[inline(never)]
#[allow(unsafe_op_in_unsafe_fn)]
#[allow(static_mut_refs)]
pub unsafe fn paint_stack() {
    let stack_start = &_stack_start as *const u32;
    let stack_end = &mut _stack_end as *mut u32;
    let current_sp = get_stack_pointer();
    let safe_limit = (current_sp as usize - SAFETY_MARGIN_WORDS * 4) as *const u32;

    info!("Stack start: 0x{:x}", stack_start as usize);
    info!("Stack end: 0x{:x}", stack_end as usize);

    let mut ptr = stack_end;

    while (ptr as *const u32) < safe_limit {
        ptr::write_volatile(ptr, STACK_PAINT_VALUE);
        ptr = ptr.add(1);
    }
}

#[inline(never)]
#[allow(unsafe_op_in_unsafe_fn)]
#[allow(static_mut_refs)]
pub unsafe fn measure_stack_usage() {
    let stack_start = &_stack_start as *const u32;
    let stack_end = &_stack_end as *const u32;

    let mut ptr = stack_end;
    let mut unused_words = 0;

    while ptr < stack_start {
        if ptr::read_volatile(ptr) == STACK_PAINT_VALUE {
            unused_words += 1;
            ptr = ptr.add(1);
        } else {
            break;
        }
    }

    let total_bytes = (stack_start as usize) - (stack_end as usize);
    let unused_bytes = unused_words * 4;
    let used_bytes = total_bytes - unused_bytes;

    info!("Stack: {} (0x{:x}) / {} (0x{:x}) bytes used", used_bytes, used_bytes, total_bytes, total_bytes);
    info!("Free: {} (0x{:x}) bytes", unused_bytes, unused_bytes);
}

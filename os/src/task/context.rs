//! Implementation of [`TaskContext`]
use crate::trap::trap_return;
const BIG_STRIDE:isize=1000000;

#[repr(C)]
/// task context structure containing some registers
pub struct TaskContext {
    /// Ret position after task switching
    ra: usize,
    /// Stack pointer
    sp: usize,
    /// s0-11 register, callee saved
    s: [usize; 12],
    ///save runtime and taskinfo times
    pub ss: [usize; 16],
    ///priority
    pub prio:isize,
    ///stride
    pub stride:usize,
    ///pass
    pub pass:usize,
}

impl TaskContext {
    /// Create a new empty task context
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
            ss: [0; 16],
            prio: 16,
            stride: 0,
            pass: (BIG_STRIDE/16)as usize,
        }
    }
    /// Create a new task context with a trap return addr and a kernel stack pointer
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
            ss: [0; 16],
            prio: 16,
            stride: 0,
            pass: (BIG_STRIDE/16)as usize,
        }
    }
}

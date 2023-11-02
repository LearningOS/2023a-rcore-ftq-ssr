//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of `PID_ALLOCATOR` allocates pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod id;
mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use super::mm::{VirtAddr,PageTable,MapPermission,};
use crate::timer::get_time_us;
use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor,
};
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        panic!("All applications completed!");
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// read syscall
const SYSCALL_READ: usize = 63;
/// write syscall
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// setpriority syscall
const SYSCALL_SET_PRIORITY: usize = 140;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// getpid syscall
const SYSCALL_GETPID: usize = 172;
/// sbrk syscall
const SYSCALL_SBRK: usize = 214;
/// munmap syscall
const SYSCALL_MUNMAP: usize = 215;
/// fork syscall
const SYSCALL_FORK: usize = 220;
/// exec syscall
const SYSCALL_EXEC: usize = 221;
/// mmap syscall
const SYSCALL_MMAP: usize = 222;
/// waitpid syscall
const SYSCALL_WAITPID: usize = 260;
/// spawn syscall
const SYSCALL_SPAWN: usize = 400;
/// taskinfo syscall
const SYSCALL_TASK_INFO: usize = 410;
/// add_syscall_times
pub fn add_syscall_times(syscall_id:usize){

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut inner.task_cx as *mut TaskContext;
    unsafe{
        match syscall_id {
            SYSCALL_READ => (*task_cx_ptr).ss[1]+=1,
            SYSCALL_WRITE => (*task_cx_ptr).ss[2]+=1,
            SYSCALL_EXIT => (*task_cx_ptr).ss[3]+=1,
            SYSCALL_YIELD => (*task_cx_ptr).ss[4]+=1,
            SYSCALL_GETPID => (*task_cx_ptr).ss[5]+=1,
            SYSCALL_FORK => (*task_cx_ptr).ss[6]+=1,
            SYSCALL_EXEC => (*task_cx_ptr).ss[7]+=1,
            SYSCALL_WAITPID => (*task_cx_ptr).ss[8]+=1,
            SYSCALL_GET_TIME => (*task_cx_ptr).ss[9]+=1,
            SYSCALL_TASK_INFO => (*task_cx_ptr).ss[10]+=1,
            SYSCALL_MMAP => (*task_cx_ptr).ss[11]+=1,
            SYSCALL_MUNMAP => (*task_cx_ptr).ss[12]+=1,
            SYSCALL_SBRK => (*task_cx_ptr).ss[13]+=1,
            SYSCALL_SPAWN => (*task_cx_ptr).ss[14]+=1,
            SYSCALL_SET_PRIORITY => (*task_cx_ptr).ss[15]+=1,
            _ => panic!("Unsupported syscall_id: {}", syscall_id),
        }
    }
}
/// get_current_status
pub fn get_current_status() -> TaskStatus{
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    inner.task_status
}
/// get_current_syscall_times
pub fn get_current_syscall_times()->[u32;500]{
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let mut tmp_call:[u32;500]=[0;500];
    let task_cx = &mut inner.task_cx as *mut TaskContext;
    unsafe{
        tmp_call[SYSCALL_READ] = (*task_cx).ss[1] as u32;
        tmp_call[SYSCALL_WRITE] = (*task_cx).ss[2] as u32;
        tmp_call[SYSCALL_EXIT] = (*task_cx).ss[3] as u32;
        tmp_call[SYSCALL_YIELD] = (*task_cx).ss[4] as u32;
        tmp_call[SYSCALL_GETPID] = (*task_cx).ss[5] as u32;
        tmp_call[SYSCALL_FORK] = (*task_cx).ss[6] as u32;
        tmp_call[SYSCALL_EXEC] = (*task_cx).ss[7] as u32;
        tmp_call[SYSCALL_WAITPID] = (*task_cx).ss[8] as u32;
        tmp_call[SYSCALL_GET_TIME] = (*task_cx).ss[9] as u32;
        tmp_call[SYSCALL_TASK_INFO] = (*task_cx).ss[10] as u32;
        tmp_call[SYSCALL_MMAP] = (*task_cx).ss[11] as u32;
        tmp_call[SYSCALL_MUNMAP] = (*task_cx).ss[12] as u32;
        tmp_call[SYSCALL_SBRK] = (*task_cx).ss[13] as u32;
        tmp_call[SYSCALL_SPAWN] = (*task_cx).ss[14] as u32;
        tmp_call[SYSCALL_SET_PRIORITY] = (*task_cx).ss[15] as u32;
    }
    tmp_call
}
/// get_current_runtime
pub fn get_current_runtime() -> usize{
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let task_cx = &mut inner.task_cx as *mut TaskContext;
    unsafe{
        get_time_us()/1000-(*task_cx).ss[0]
    }
}
/// create_current_maparea
pub fn create_current_maparea(start: VirtAddr,end: VirtAddr,perms:MapPermission){
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.memory_set.insert_framed_area(start, end, perms);
}
/// remove_current_maparea
pub fn remove_current_maparea(page_table: &mut PageTable,start: VirtAddr,end: VirtAddr)->bool{
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.memory_set.remove_area(page_table, start,end)
}
const BIG_STRIDE:isize=1000000;
/// set_priority
pub fn set_priority(prio: isize){
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let task_cx = &mut inner.task_cx as *mut TaskContext;
    unsafe{
        (*task_cx).prio=prio;
        (*task_cx).pass=(BIG_STRIDE/prio) as usize;
    }
}
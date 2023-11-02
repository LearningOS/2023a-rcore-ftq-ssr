//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,TaskControlBlock,
        get_current_status, get_current_runtime,get_current_syscall_times,
        create_current_maparea,
        remove_current_maparea,
        set_priority,
    },
    timer::get_time_us,
    mm::{PageTable,VirtAddr,
        MapPermission,},
    config::PAGE_SIZE,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // -1
    let us=get_time_us();
    let ptr=_ts as usize;
    *translated_refmut(current_user_token(),ptr as *mut _)=us / 1_000_000;
    *translated_refmut(current_user_token(), (ptr+8) as *mut _)=us % 1_000_000;
    0
}

// const SYSCALL_WRITE: usize = 64;
// /// exit syscall
// const SYSCALL_EXIT: usize = 93;
// /// yield syscall
// const SYSCALL_YIELD: usize = 124;
// /// gettime syscall
// const SYSCALL_GET_TIME: usize = 169;
// /// sbrk syscall
// /// taskinfo syscall
// const SYSCALL_TASK_INFO: usize = 410;
/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // -1
    let _status=get_current_status();
    let _syscall_times=get_current_syscall_times();
    let _run_time=get_current_runtime();
    // println!("SYSCALL_WRITE={}",_syscall_times[SYSCALL_WRITE]);
    // println!("SYSCALL_EXIT={}",_syscall_times[SYSCALL_EXIT]);
    // println!("SYSCALL_YIELD={}",_syscall_times[SYSCALL_YIELD]);
    // println!("SYSCALL_GET_TIME={}",_syscall_times[SYSCALL_GET_TIME]);
    // println!("SYSCALL_TASK_INFO={}",_syscall_times[SYSCALL_TASK_INFO]);
    *translated_refmut(current_user_token(),_ti as *mut _)=TaskInfo{
        status: _status,
        syscall_times:_syscall_times,
        time: _run_time
    };
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // -1
    if _port & !0x7 != 0||_port & 0x7 == 0||_start%PAGE_SIZE!=0 {return -1;}
    let page_table = PageTable::from_token(current_user_token());
    let mut flags=MapPermission::U;
    if _port&(1<<0)!=0 {flags|=MapPermission::R;}
    if _port&(1<<1)!=0 {flags|=MapPermission::W;}
    if _port&(1<<2)!=0 {flags|=MapPermission::X;}
    for i in (_start.._start+_len).step_by(PAGE_SIZE).into_iter(){
        if let Some(x)=page_table.translate(VirtAddr::from(i).floor()) {
            if x.is_valid() {return -1;}
        }
    }
    create_current_maparea(VirtAddr::from(_start),VirtAddr::from(_start+_len),flags);
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // -1
    if _start%PAGE_SIZE!=0 {return -1;}
    let mut page_table = PageTable::from_token(current_user_token());
    for i in (_start.._start+_len).step_by(PAGE_SIZE).into_iter(){
        if let None=page_table.translate(VirtAddr::from(i).floor()) {return -1;}
    }
    if remove_current_maparea(&mut page_table, VirtAddr::from(_start),VirtAddr::from(_start+_len))==false {return -1;}
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let current_task = current_task().unwrap();
        let new_task = Arc::new(TaskControlBlock::new(data));
        let new_pid: usize=new_task.pid.0;
        // new_task.exec(data);
        let mut parent_inner = current_task.inner_exclusive_access();
        parent_inner.children.push(new_task.clone());
        let mut child_inner = new_task.inner_exclusive_access();
        child_inner.parent=Some(Arc::downgrade(&current_task));
        drop(child_inner);
        add_task(new_task);
        new_pid as isize
    } else {
         -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _prio<=1 {
        -1
    }else{
        set_priority(_prio);
        _prio
    }
}

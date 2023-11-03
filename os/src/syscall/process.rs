//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    task::{
        change_program_brk, current_user_token,exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
    },
    timer::get_time_us,
};
use crate::task::{get_current_status, get_current_runtime,get_current_syscall_times};
// use crate::mm::translated_mut;
use crate::mm::{change_mut_usize,create_mmap,remove_mmap,//change_mut_u8,change_mut_u32,};
    translated_byte_buffer};
// use alloc::vec;
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
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // trace!("kernel: sys_get_time");
    // let us = get_time_us();
    // unsafe {
    //     *_ts = TimeVal {
    //         sec: us / 1_000_000,
    //         usec: us % 1_000_000,
    //     };
    // }
    // 0
    let us = get_time_us();
    {
        // let tmp: &mut u8=translated_mut(current_user_token(),_ts as *const u8);
        // *tmp=TimeVal{sec:us / 1_000_000,usec:us % 1_000_000}as usize;
        // let sec=translated_mut(current_user_token(),((*_ts).sec) as *const u8);
        // *sec = (us / 1_000_000) as u8;
        // let usec=translated_mut(current_user_token(),((*_ts).usec) as *const u8);
        // *usec = (us % 1_000_000) as u8;
       // let sec=buffer[0][0..1][0];
        // let ts_sec:&[u8]=&sec[0..1];
        // ts_sec[0] = (us / 1_000_000) as u8;
        // let mut buffer: alloc::vec::Vec<&mut [u8]> = translated_byte_buffer(current_user_token(), (*_ts).sec as *const u8, 1);
        let mut ptr=_ts as usize;
        change_mut_usize(current_user_token(),ptr,us / 1_000_000);//与余智超,叶可禾交流了此处变量虚拟地址转物理地址的实现
        ptr+=8;
        change_mut_usize(current_user_token(),ptr,us % 1_000_000);
    }
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
// use alloc::vec;
use alloc::vec::Vec;
/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    // trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    // -1
    // println!("SYSCALL_WRITE={}",_syscall_times[SYSCALL_WRITE]);
    // println!("SYSCALL_EXIT={}",_syscall_times[SYSCALL_EXIT]);
    // println!("SYSCALL_YIELD={}",_syscall_times[SYSCALL_YIELD]);
    // println!("SYSCALL_GET_TIME={}",_syscall_times[SYSCALL_GET_TIME]);
    // println!("SYSCALL_TASK_INFO={}",_syscall_times[SYSCALL_TASK_INFO]);
    // trace!("kernel: sys_task_info");
    // let _status=get_current_status();
    // let _syscall_times=get_current_syscall_times();
    // let _run_time=get_current_runtime();
    // println!("runtime={}",_run_time);
    // println!("{:?}",_status as u8);
    // {
    //     let mut ptr=_ti as usize;
    //     change_mut_u8(current_user_token(),ptr,_status as u8);
    //     ptr+=1;
    //     for i in 0..500{
    //         change_mut_u32(current_user_token(),ptr,_syscall_times[i]);
    //         ptr+=4;
    //     }
    //     change_mut_usize(current_user_token(),ptr,_run_time);
    // };
    // 0
    trace!("kernel: sys_task_info");
    let _status=get_current_status();
    let _syscall_times=get_current_syscall_times();
    let _run_time=get_current_runtime();
    let _taskinfo_len: usize = core::mem::size_of::<TaskInfo>();
    let mut buffers: Vec<&mut [u8]> =
        translated_byte_buffer(current_user_token(), _ti as *const u8,_taskinfo_len);
    let _taskinfo: TaskInfo = TaskInfo{
        status: _status,
        syscall_times:_syscall_times,
        time: _run_time
    };
    let _taskinfo_slice: &[u8 ] = unsafe{ core::slice::from_raw_parts(&_taskinfo as *const _ as *const u8,_taskinfo_len)};
    buffers[0][.._taskinfo_len].copy_from_slice(_taskinfo_slice);
    0
        
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    // trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    trace!("kernel: sys_mmap has IMPLEMENTED !");
    if _port & !0x7 != 0||_port & 0x7 == 0||_start%PAGE_SIZE!=0 {return -1;}
    // start 没有按页大小对齐
    // port & !0x7 != 0 (port 其余位必须为0)
    // port & 0x7 = 0 (这样的内存无意义)
    // [start, start + len) 中存在已经被映射的页
    // 物理内存不足
    // println!("start: {}   len:{}   port:{}",_start,_len,_port);
    create_mmap(current_user_token(),_start,_len,_port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if _start%PAGE_SIZE!=0 {return -1;}
    // if _port & !0x7 != 0||_port & 0x7 == 0||_start%PAGE_SIZE!=0 {return -1;}
    //[start, start + len) 中存在未被映射的虚存。
    remove_mmap(current_user_token(),_start,_len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

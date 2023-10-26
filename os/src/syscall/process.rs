//! Process management syscalls
// use std::time::SystemTime;
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{exit_current_and_run_next, suspend_current_and_run_next,get_current_status, get_current_runtime,get_current_syscall_times,TaskStatus},
    timer::{get_time_us}, sync::UPSafeCell, 
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
use lazy_static::*;
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
lazy_static!{
    pub static ref TASK_INFO: UPSafeCell<TaskInfo> =unsafe{
        UPSafeCell::new({
            TaskInfo {
                status: TaskStatus::UnInit,
                syscall_times: [0; MAX_SYSCALL_NUM],
                time: 0,
            }
        })
    };
}
// impl UPSafeCell<TaskInfo>{
//     fn get_current_syscall_times(&self)->[u32; MAX_SYSCALL_NUM]{
//         self.syscall_times
//     }
//     // fn get_current_runtime(&self)->usize{
//     //     self.time
//     // }
//     // pub fn set_time(&self){
//     //      self.time=get_time_ms();
//     // }
//     pub fn add_syscall_times(&self,syscall_id:usize){
//         let mut syscall_times=self.syscall_times;
//         syscall_times[syscall_id]+=1;
//     }
// }
/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// /// write syscall
// const SYSCALL_WRITE: usize = 64;
// /// yield syscall
// const SYSCALL_YIELD: usize = 124;
// /// gettime syscall
// const SYSCALL_GET_TIME: usize = 169;
// /// taskinfo syscall
// const SYSCALL_TASK_INFO: usize = 410;
// /// exit syscall
// const SYSCALL_EXIT: usize = 410;
/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let _status=get_current_status();
    let _syscall_times=get_current_syscall_times();
    let _run_time=get_current_runtime();
    // println!("SYSCALL_WRITE={}",_syscall_times[SYSCALL_WRITE]);
    // println!("SYSCALL_EXIT={}",_syscall_times[SYSCALL_EXIT]);
    // println!("SYSCALL_YIELD={}",_syscall_times[SYSCALL_YIELD]);
    // println!("SYSCALL_GET_TIME={}",_syscall_times[SYSCALL_GET_TIME]);
    // println!("SYSCALL_TASK_INFO={}",_syscall_times[SYSCALL_TASK_INFO]);
    println!("runtime={}",_run_time);
    unsafe{
        *_ti=TaskInfo{
            status:_status,
            syscall_times:_syscall_times,
            time:_run_time,
        }
    };
    0
}

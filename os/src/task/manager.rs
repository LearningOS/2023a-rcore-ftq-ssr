//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
// use alloc::collections::Vec;
use alloc::sync::Arc;
use lazy_static::*;

// use crate::task::TaskContext;

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    // ready_queue: VecDeque<Arc<TaskControlBlock>>,
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // let mut minest_stride: usize=20000000000usize;
        // let mut new_vec: VecDeque<Arc<TaskControlBlock>>=VecDeque::new();
        // while !self.ready_queue.is_empty() {
        //     let now=&self.ready_queue.front().unwrap();
        //     if now.inner_exclusive_access().task_cx.stride<=minest_stride {
        //         minest_stride=now.inner_exclusive_access().task_cx.stride;
        //     }
        //     new_vec.push_back(self.ready_queue.front().unwrap());
        // }
        // let mut flag:bool=false;
        // let mut ret_task: Option<Arc<TaskControlBlock>>=None;
        // while !new_vec.is_empty() {
        //     let now=new_vec.pop_front().unwrap();
        //     if flag==false&&now.inner_exclusive_access().task_cx.stride==minest_stride {
        //         flag=true;
        //         now.inner_exclusive_access().task_cx.stride+=now.inner_exclusive_access().task_cx.pass;
        //         ret_task=Some(now);
        //     }else {
        //         self.ready_queue.push_back(now);
        //     }
        // }
        // ret_task

        let mut min_id:usize=0;
        for i in 1..self.ready_queue.len(){
            if self.ready_queue.get(i).unwrap().inner_exclusive_access().task_cx.stride
                <self.ready_queue.get(min_id).unwrap().inner_exclusive_access().task_cx.stride {
                min_id=i;
            }
        }
        // let clone_task=min_task.clone();
        let min_task=self.ready_queue.remove(min_id);
        // min_task.unwrap().inner_exclusive_access().task_cx.stride+=min_task.unwrap().inner_exclusive_access().task_cx.pass;
        min_task
        // self.ready_queue.pop_front()
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

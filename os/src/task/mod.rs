//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;
use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
// use crate::syscall::process::TASK_INFO;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// Inner of Task Manager
pub struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}
/// write syscall
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// taskinfo syscall
const SYSCALL_TASK_INFO: usize = 410;
impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        // (task0.task_cx).ss=[0;6];
        (task0.task_cx).ss[0]=get_time_ms();
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }
    /// add_syscall_times
    pub fn add_syscall_times(&self,syscall_id:usize){
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let tasknow = &mut inner.tasks[current];
        match syscall_id {
            SYSCALL_WRITE => (tasknow.task_cx).ss[1]+=1,
            SYSCALL_EXIT => (tasknow.task_cx).ss[2]+=1,
            SYSCALL_YIELD => (tasknow.task_cx).ss[3]+=1,
            SYSCALL_GET_TIME => (tasknow.task_cx).ss[4]+=1,
            SYSCALL_TASK_INFO => (tasknow.task_cx).ss[5]+=1,
            _ => panic!("Unsupported syscall_id: {}", syscall_id),
        }
    }
    /// get_current_status
    fn get_current_status(&self) -> TaskStatus{
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status
    }
    /// get_current_syscall_times
    pub fn get_current_syscall_times(&self)->[u32;500]{
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let tasknow = &mut inner.tasks[current];
        let mut tmp_call:[u32;500]=[0;500];
        tmp_call[SYSCALL_WRITE] = (tasknow.task_cx).ss[1] as u32;
        tmp_call[SYSCALL_EXIT] = (tasknow.task_cx).ss[2] as u32;
        tmp_call[SYSCALL_YIELD] = (tasknow.task_cx).ss[3] as u32;
        tmp_call[SYSCALL_GET_TIME] = (tasknow.task_cx).ss[4] as u32;
        tmp_call[SYSCALL_TASK_INFO] = (tasknow.task_cx).ss[5] as u32;
        tmp_call
    }
    /// get_current_runtime
    fn get_current_runtime(&self) -> usize{
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let tasknow = &mut inner.tasks[current];
        get_time_ms()-(tasknow.task_cx).ss[0]
    }
    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            let tasknt = &mut inner.tasks[next];
            // tasknt.task_status = TaskStatus::Running;
            // (tasknt.task_cx).ss=[0;6];
            if (tasknt.task_cx).ss[0]==0 {(tasknt.task_cx).ss[0]=get_time_ms();}
            // println!("task {};runtime={}",current,(tasknt.task_cx).ss[0]);
            // for i in 0..12{
            //     println!("{}:{}",i,(tasknt.task_cx).s[i]);
            // }
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// return the current status
pub fn get_current_status()->TaskStatus{
    TASK_MANAGER.get_current_status()
}
/// return the current syscall_times
pub fn get_current_syscall_times()->[u32;500]{
    TASK_MANAGER.get_current_syscall_times()
}
/// return the current runtime
pub fn get_current_runtime()->usize{
    TASK_MANAGER.get_current_runtime()
}


/// add_syscall_times
pub fn add_syscall_times(syscall_id:usize){
    TASK_MANAGER.add_syscall_times(syscall_id);
}
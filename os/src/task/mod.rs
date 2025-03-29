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
use crate::mm::{PageTableEntry, SimpleRange};
use crate::loader::{get_app_data, get_num_app};
use crate::mm::{VirtPageNum,VirtAddr,MapPermission};
//use crate::mm::{MapPermission, VirtPageNum};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
//use alloc::collections::btree_map::Cursor;
use alloc::vec::Vec;
use lazy_static::*;
//use riscv::addr::VirtAddr;
//use crate::mm::VirtAddr;
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

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
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

impl TaskManager {
    ///解引用
    pub fn umap_memoryset(&self,start:VirtAddr,end:VirtAddr )->i32{
        let start_vpn=start.floor();
        let end_vpn=end.ceil();
        let inner =& mut self.inner.exclusive_access();
        let current=inner.current_task;
        
        for vpn in SimpleRange::new(start_vpn, end_vpn){
            //println!("umap start_vpn is {:?}",vpn);
            if let Some(pte)=inner.tasks[current].memory_set.translate(vpn){
                //println!("pte is  {:?} start_vpn is {:?}",pte,start_vpn);
                if !pte.is_valid(){
                    return -1;
                }
                
            }else {
               return -1;
            }
            inner.tasks[current].memory_set.page_table_umap(vpn);
            if let Some(pte)=inner.tasks[current].memory_set.translate(vpn){
                if pte.is_valid(){
                //println!("umap is err {:?} start_vpn is {:?}",pte,start_vpn);
                }
            }
        }
        0
    }    
    ///创造新map区域
    pub fn creat_new_map_area(& self,start_va: VirtAddr, end_va: VirtAddr, perm: MapPermission){
        let inner=& mut self.inner.exclusive_access();
        let current=inner.current_task;
        inner.tasks[current].memory_set.insert_framed_area(start_va, end_va, perm);
    }
    ///检查页帧是否已分配
    pub fn curret_fram_is_empty(&self,vpn:VirtPageNum)->Option<PageTableEntry>{
        let inner=& mut self.inner.exclusive_access();
        let current=inner.current_task;
        inner.tasks[current].memory_set.translate(vpn)
    }
    ///测试函数
    pub fn read(&self)->usize{
        let inner=& mut self.inner.exclusive_access();
        let current=inner.current_task;
        current
    }

    ///调用次数读取
    pub fn read_syycall(&self,_id:usize)->usize{
        
        let inner =self.inner.exclusive_access();
        let current=inner.current_task;
        inner.tasks[current].syscall_times[_id]
    }
    ///trace跟踪系统调
    pub fn syscall_updata(& self ,_id:usize){
        let inner=&mut self.inner.exclusive_access();
        let current=inner.current_task;
        //if current!=0{
        //println!("the current is {} id{}",current,_id);
        //}
        inner.tasks[current].syscal_record(_id);
    }
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
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

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
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

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

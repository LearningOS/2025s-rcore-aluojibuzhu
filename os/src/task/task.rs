//! Types related to task management

use super::TaskContext;
/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    ///syscall times
    pub system_call_times:[u32;500],
}

impl TaskControlBlock {
    ///更新当前任务系统调用次数
    pub fn syscall_times_updata(& mut self,_id :usize){
        self.system_call_times[_id]+=1;
    }
    ///
    pub fn read_syscall_times(& mut self,_id :usize)->isize{
    self.system_call_times[_id] as isize
    }
}
/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}

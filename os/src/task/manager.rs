//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use crate::task::current_task;
use crate::mm::VirtAddr;
use crate::mm::MapPermission;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
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
    /*pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        //任务队列为空
            if let Some(index) =self.ready_queue.iter().position(|tcb| {
                tcb.inner_exclusive_access().schedule_priority.stride>=task.inner_exclusive_access().schedule_priority.stride
            }
                )
                    {
                    self.ready_queue.insert(index, task);
                    }
                    else {
                    self.ready_queue.push_back(task);
                    }

    }
    */
    /// Take a process out of the ready queue
   /*pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    }*/

    /// Add process back to ready queue

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
  
    /// Take a process out of the ready queue 
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let mut min=0;
        let mut index=0;
       for (index_iter,task) in  self.ready_queue.iter().enumerate(){
            if task.inner_exclusive_access().schedule_priority.stride<=min{
                index=index_iter;
                min=task.inner_exclusive_access().schedule_priority.stride;
            }
       }
       if let  Some(fech_task)=self.ready_queue.remove(index){
            fech_task.inner_exclusive_access().schedule_priority.tcb_pass_strde();
            return Some(fech_task);
       }else{
        return None;
       }
       
    }

    ///创造新map区域
    pub fn creat_new_map_area(& self,start_va: VirtAddr, end_va: VirtAddr, perm: MapPermission){
        let inner=& mut self.ready_queue[current_task().unwrap().pid.0].clone();
        inner.inner_exclusive_access().memory_set.insert_framed_area(start_va, end_va, perm);
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

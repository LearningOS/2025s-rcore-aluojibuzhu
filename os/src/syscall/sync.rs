use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use crate::task::TASK_MANAGER;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.available[0]+=1;//修改可分配数组
        process_inner.mutex_list.len() as isize - 1
    }
    
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    //记录请求锁
    if let Some(thread)=current_task(){
        let mut inner= thread.inner_exclusive_access();
        let res=inner.res.as_mut().unwrap();
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        //死锁检测
        if process_inner.dead_lock_check==true{
            let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
            let _wait_list =mutex.queue_list();
            let  allocation=process_inner.allocation.clone();
            let mut need =process_inner.need.clone();
            let task_inner=TASK_MANAGER.exclusive_access();
            //let ready_queue=task_inner.ready_list();
            let  work=process_inner.available[0];
            need[res.tid][0]+=1;
            //debug!("the allocation is {:?}",allocation);
            //debug!("the need is {:?}",need);
            //debug!("the available is {:?}",process_inner.available);
                if (need[res.tid][0]-allocation[res.tid][0])<=work{
                    process_inner.need[res.tid][0]+=1;
                    process_inner.available[0]-=1;
                    process_inner.allocation[res.tid][0]+=1;
                    //println!("the available is {:?}",process_inner.available);
                }else {
                    //debug!("deadlock err");
                    println!("avaiable is {:?}",process_inner.available[0]);
                    println!("need is {:?}",process_inner.need);
                    return -0xDEAD;
                }
                drop(task_inner);
            //锁分配
            }
            let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
            drop(process_inner);
            drop(inner);
            mutex.lock();
    }
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    if let Some(thread)=current_task(){
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    mutex.unlock();
    
        let mut inner= thread.inner_exclusive_access();
        let res=inner.res.as_mut().unwrap();
        process_inner.available[0]+=1;
        process_inner.allocation[res.tid][0]-=1;
    }
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        //可用资源更新
        process_inner.available[id+1]=res_count as isize;
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        //可用资源更新
        let id=process_inner.semaphore_list.len() - 1;
        process_inner.available[id+1]=res_count as isize;
        id
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    if let Some(thread)=current_task(){
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
        sem.up();
        let mut inner= thread.inner_exclusive_access();
        let res=inner.res.as_mut().unwrap();
        process_inner.available[sem_id+1]+=1;
        //println!("the sem id is {} can the available is return ? {} the thread is {}",sem_id,process_inner.available[sem_id+1],res.tid);
        process_inner.allocation[res.tid][sem_id+1]-=1;
    }
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    //记录请求锁
    if let Some(thread)=current_task(){
        let mut inner= thread.inner_exclusive_access();
        let res=inner.res.as_mut().unwrap();
        let process = current_process();
        let  mut process_inner = process.inner_exclusive_access();
        //debug!("process_inner.dead_lock_check {:?}",process_inner.dead_lock_check);
        //死锁检测
        if process_inner.dead_lock_check==true{
            let semaphore = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
            let _wait_list =semaphore.queue_list();
            let task_inner=TASK_MANAGER.exclusive_access();
            let ready_queue=task_inner.ready_list();
            let  allocation=process_inner.allocation.clone();
            let mut need =process_inner.need.clone();
            let  work=process_inner.available[sem_id+1];
            //let mut finsh:[bool;10]=[false;10];
            need[res.tid][sem_id+1]+=1;
            //debug!("the allocation is {:?}",allocation);
            //debug!("the need is {:?}",need);
            //debug!("the available is {:?}",process_inner.available);
            //缺少可以继续分配的资源
            //println!("the sem id is {}",sem_id);
            //假设分配当前锁
            //    debug!("ok");
            if (need[res.tid][sem_id+1]-allocation[res.tid][sem_id+1])<=work{
                process_inner.need[res.tid][sem_id+1]+=1;
                process_inner.available[sem_id+1]-=1;
                process_inner.allocation[res.tid][sem_id+1]+=1;
                //println!("the available is {:?}",process_inner.available);
            }else if ready_queue.iter().find(|task| **task!=0).is_none(){
                //debug!("deadlock err");
                //println!("the task read_queue is {:?}",ready_queue);
                return -0xDEAD;
            }
                let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
                drop(task_inner);
                drop(process_inner);
                drop(inner);
                sem.down();
        }
    //记录请求锁
    }
    0
}

/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
        let process = current_process();
        let mut  process_inner = process.inner_exclusive_access();
        let task_inner=TASK_MANAGER.exclusive_access();
        let ready_queue=task_inner.ready_list();
        println!("task quue is {:?}",ready_queue);
        process_inner.dead_lock_check=true;
    0
}

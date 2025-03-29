//! Process management syscalls
use core::usize;

//use riscv::register::fcsr::Flag;
use crate::mm::{MapPermission, SimpleRange};
use crate::config::PAGE_SIZE;
use crate::syscall::{SYSCALL_EXIT, SYSCALL_GET_TIME, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_SBRK, SYSCALL_TRACE, SYSCALL_YIELD};
use crate::task::{change_program_brk, current_user_token, exit_current_and_run_next, suspend_current_and_run_next};
use crate::mm::{translate_flag, translate_timeval, translate_usize, PTEFlags, VirtAddr};
use crate::timer::get_time_us;
use crate::task::TASK_MANAGER;
#[repr(C)]
#[derive(Debug)]
///
pub struct TimeVal {
    ///
    pub sec: usize,
    ///
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    TASK_MANAGER.syscall_updata(SYSCALL_EXIT );
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    TASK_MANAGER.syscall_updata(SYSCALL_YIELD );
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    TASK_MANAGER.syscall_updata(SYSCALL_GET_TIME );
    let time_weget=translate_timeval(current_user_token(),_ts);
    *time_weget=TimeVal{
        sec:get_time_us()/1000000,
        usec:get_time_us()%1000000,
    };
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// 不要在很烦的时候写代码
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token=current_user_token();
   // let trace_vaddr=translate_usize(token, _trace_request);
    if let Some(id_vaddr)=translate_usize(token, _id){
    //let data_vaddr=translate_usize(token, _data);
    TASK_MANAGER.syscall_updata(SYSCALL_TRACE );
        match _trace_request {
            0=>{
                match translate_flag(token,_id) {
                    None=>{//println!("0 the flag is err{}",_id);
                    return -1;},
                    Some(pte)=>{
                        //println!("{:?},{}",pte);
                            if pte&PTEFlags::R==PTEFlags::R
                            {
                                return *id_vaddr as isize;
                            }else{
                                //println!("读标志为不对");
                                return -1;
                            }
                    } 
                }
            },
            1=>{
                match translate_flag(token,_id) {
                    None=>{//println!("1 the flag is err{}",_id);
                    return -1;},
                    Some(pte)=>{
                            if pte&PTEFlags::W==PTEFlags::W
                            {
                             *id_vaddr =_data;
                             return 0;
                            }else{
                                //println!("写标志为不对");
                                return -1;
                            }
                    } 
                }
            }
            2=>{
                return TASK_MANAGER.read_syycall(_id) as isize
            }
            _=>return -1,
        }
    }
    else {
        //println!("the address is none id is{}",_id);
        return -1;
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    TASK_MANAGER.syscall_updata(SYSCALL_MMAP );
    
    if (_start%PAGE_SIZE!=0)||
        _port & !0x7 != 0||
        _port & 0x7 == 0
        {
            //println!("arguments is err");
            return -1
        }
    let start_vpn=VirtAddr(_start).floor();
    let end_vpn=VirtAddr(_start+_len).ceil();
    let vpns=SimpleRange::new(start_vpn,end_vpn);

    for vpn in vpns{
        if let Some(pte)=TASK_MANAGER.curret_fram_is_empty(vpn){
            if pte.is_valid(){
                println!("pte.is_valid{:?}",vpn);
                return -1;
            }
        }
    }
     
    TASK_MANAGER.creat_new_map_area(VirtAddr(_start), 
    VirtAddr(_start+_len), 
    MapPermission::from_bits_truncate((_port<<1) as u8)|MapPermission::U);
    return 0

}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    TASK_MANAGER.syscall_updata(SYSCALL_MUNMAP );

    if _start%PAGE_SIZE!=0
        {
            //println!("arguments is err");
            return -1
        }

    let start_vpn=VirtAddr(_start);
    let end_vpn=VirtAddr(_start+_len);

    if TASK_MANAGER.umap_memoryset(start_vpn, end_vpn)==-1{
        return -1;
    }


    return 0;
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    TASK_MANAGER.syscall_updata(SYSCALL_SBRK );
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

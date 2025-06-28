use std::ffi::CString;

use log::error;
use pyo3::{prelude::*, types::PyDict};

const STACK_THREADS: &str = include_str!("stack_get_threads.py");

pub fn get_python_stacks(tid: i32) -> Option<String> {
    log::debug!("Collecting python backtrace for TID: {:?}", tid);

    Python::with_gil(|py| {
        log::debug!("Start calling backtrace for TID: {:?}", tid);
        let global = PyDict::new(py);
        global.set_item("tid", tid).unwrap_or_else(|err| {
            error!("Failed to set 'tid' in Python global dict: {}", err);
        });
        let script_cstr = CString::new(STACK_THREADS).unwrap_or_default();
        if let Err(err) = py.run(&script_cstr, Some(&global), Some(&global)) {
            error!("Failed to execute Python stack dump script: {}", err);
            return None;
        }
        match global.get_item("retval") {
            Ok(Some(frames_str)) => frames_str.extract::<String>().ok(),
            Ok(None) => {
                error!("Python stack dump script did not return 'retval'");
                None
            }
            Err(err) => {
                error!("Failed to get 'retval' from Python stack dump: {}", err);
                None
            }
        }
    })
}

// use nix::libc::{pthread_self, write, SIGUSR2, STDERR_FILENO};
// use pyo3::ffi;
// use std::sync::atomic::{AtomicBool, Ordering};

// static BUSY: AtomicBool = AtomicBool::new(false);

// /// 信号处理器 - 核心逻辑
// extern "C" fn dump_stack_handler(_: i32) {
//     // 防重入
//     if BUSY.swap(true, Ordering::Acquire) {
//         return;
//     }

//     unsafe {
//         // 尝试获取Python线程状态
//         if let Some(frames) = get_python_frames() {
//             print_frames(&frames);
//         } else {
//             write_str(b"[ERROR] Cannot get Python stack\n");
//         }
//     }

//     BUSY.store(false, Ordering::Release);
// }

// /// 获取Python调用栈
// unsafe fn get_python_frames() -> Option<Vec<Frame>> {
//     // 1. 尝试从GIL获取
//     let tstate = ffi::PyGILState_GetThisThreadState();
//     if !tstate.is_null() {
//         return extract_frames(tstate);
//     }

//     // 2. 尝试从当前线程获取
//     #[cfg(Py_3_9)]
//     {
//         // Python 3.9+ 提供了获取当前线程状态的API
//         let get_current = dlsym!("_PyThreadState_GetCurrent");
//         if let Some(func) = get_current {
//             let tstate = func();
//             if !tstate.is_null() {
//                 return extract_frames(tstate);
//             }
//         }
//     }

//     None
// }

// /// 从线程状态提取帧信息
// unsafe fn extract_frames(tstate: *mut ffi::PyThreadState) -> Option<Vec<Frame>> {
//     let mut frames = Vec::new();
//     let mut current = (*tstate).frame;

//     // 遍历调用栈（限制深度防止死循环）
//     for _ in 0..50 {
//         if current.is_null() || !is_valid_ptr(current as *const u8) {
//             break;
//         }

//         if let Some(frame) = read_frame(current) {
//             frames.push(frame);
//         }

//         current = (*current).f_back;
//     }

//     if frames.is_empty() {
//         None
//     } else {
//         Some(frames)
//     }
// }

// /// 读取单个帧信息
// unsafe fn read_frame(frame: *mut ffi::PyFrameObject) -> Option<Frame> {
//     let code = ffi::PyFrame_GetCode(f);
//     let lineno = ffi::PyFrame_GetLineNumber(frame);
//     let filename = ffi::PyCode(code);
//     let 
//     // let code = (*frame).f_code;
//     // if code.is_null() || !is_valid_ptr(code as *const u8) {
//     //     return None;
//     // }

//     Some(Frame {
//         filename: read_string((*code).co_filename),
//         function: read_string((*code).co_name),
//         lineno: (*frame).f_lineno,
//     })
// }

// /// 安全读取Python字符串
// unsafe fn read_string(obj: *mut ffi::PyObject) -> String {
//     if obj.is_null() {
//         return "<null>".to_string();
//     }

//     // 只处理ASCII - 简单且安全
//     #[cfg(Py_3_8)]
//     if ffi::PyUnicode_IS_ASCII(obj) != 0 {
//         if let Some(s) = read_ascii_string(obj) {
//             return s;
//         }
//     }

//     "<non-ascii>".to_string()
// }

// /// 读取ASCII字符串
// #[cfg(Py_3_8)]
// unsafe fn read_ascii_string(obj: *mut ffi::PyObject) -> Option<String> {
//     let data = ffi::PyUnicode_DATA(obj) as *const u8;
//     let length = ffi::PyUnicode_GET_LENGTH(obj) as usize;

//     if data.is_null() || length > 200 {
//         // 限制长度
//         return None;
//     }

//     let mut bytes = Vec::with_capacity(length);
//     for i in 0..length {
//         let byte = *data.add(i);
//         if byte == 0 {
//             break;
//         }
//         bytes.push(byte);
//     }

//     String::from_utf8(bytes).ok()
// }

// /// 检查指针有效性（简单启发式）
// fn is_valid_ptr(ptr: *const u8) -> bool {
//     let addr = ptr as usize;
//     addr > 0x1000 && addr < 0x800000000000 // 用户空间地址范围
// }

// /// 打印帧信息
// unsafe fn print_frames(frames: &[Frame]) {
//     write_str(b"\n=== Python Stack Trace ===\n");
//     write_str(b"Thread: 0x");
//     write_hex(pthread_self() as usize);
//     write_str(b"\n\n");

//     for (i, frame) in frames.iter().enumerate() {
//         // 缩进
//         for _ in 0..i {
//             write_str(b"  ");
//         }

//         // File "xxx.py", line 123, in function_name
//         write_str(b"File \"");
//         write_str(frame.filename.as_bytes());
//         write_str(b"\", line ");
//         write_num(frame.lineno);
//         write_str(b", in ");
//         write_str(frame.function.as_bytes());
//         write_str(b"\n");
//     }

//     write_str(b"\n");
// }

// /// 简单的输出函数
// unsafe fn write_str(s: &[u8]) {
//     write(STDERR_FILENO, s.as_ptr() as *const _, s.len());
// }

// unsafe fn write_num(n: i32) {
//     let s = format!("{}", n);
//     write_str(s.as_bytes());
// }

// unsafe fn write_hex(n: usize) {
//     let s = format!("{:x}", n);
//     write_str(s.as_bytes());
// }

// /// 帧信息
// struct Frame {
//     filename: String,
//     function: String,
//     lineno: i32,
// }

// /// 动态加载符号的宏
// macro_rules! dlsym {
//     ($name:literal) => {{
//         use std::ffi::CString;
//         let name = CString::new($name).unwrap();
//         let sym = libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr());
//         if sym.is_null() {
//             None
//         } else {
//             Some(std::mem::transmute::<
//                 _,
//                 unsafe extern "C" fn() -> *mut ffi::PyThreadState,
//             >(sym))
//         }
//     }};
// }

// impl StackTracer {
//     /// 安装信号处理器
//     pub fn install() -> std::io::Result<()> {
//         unsafe {
//             let mut sa: libc::sigaction = std::mem::zeroed();
//             sa.sa_handler = Some(dump_stack_handler);

//             if libc::sigaction(SIGUSR2, &sa, std::ptr::null_mut()) == 0 {
//                 Ok(())
//             } else {
//                 Err(std::io::Error::last_os_error())
//             }
//         }
//     }

//     /// 触发堆栈打印（用于测试）
//     pub fn trigger() {
//         unsafe {
//             libc::raise(SIGUSR2);
//         }
//     }
// }

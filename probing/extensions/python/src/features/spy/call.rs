use crate::features::spy::python_bindings;
use crate::features::spy::PYVERSION;

use crate::features::spy::python_interpreters::{BytesObject, CodeObject, StringObject};

pub struct RawCallLocation {
    callee: usize,
    caller: usize,
    offset: i32,
}

impl RawCallLocation {
    pub fn new(callee: usize, caller: usize, offset: i32) -> RawCallLocation {
        RawCallLocation {
            callee,
            caller,
            offset,
        }
    }

    pub fn resolve(&self) -> Result<CallLocation, std::io::Error> {
        CallLocation::try_from(self)
    }

    pub fn from_frame(addr: usize) -> RawCallLocation {
        match unsafe { (PYVERSION.major, PYVERSION.minor) } {
            (3, 4) | (3, 5) | (3, 6) | (3, 7) | (3, 8) | (3, 9) | (3, 10) => {
                // Python 3.4 to 3.9
                let frame = addr as *const python_bindings::v3_10_0::_frame;
                let code = unsafe { (*frame).f_code };
                let lasti = unsafe { (*frame).f_lasti };
                RawCallLocation::new(code as usize, 0, lasti)
            }
            (3, 11) => {
                let iframe = addr as *const python_bindings::v3_11_0::_PyInterpreterFrame;
                unsafe {
                    let code = (*iframe).f_code;
                    let lasti = ((*iframe).prev_instr as *const u8).offset_from(code as *const u8);
                    RawCallLocation::new(code as usize, 0, lasti as i32)
                }
            }
            (3, 12) => {
                // Python 3.10 and later
                let iframe = addr as *const python_bindings::v3_12_0::_PyInterpreterFrame;
                unsafe {
                    let code = (*iframe).f_code;
                    let lasti = ((*iframe).prev_instr as *const u8).offset_from(code as *const u8);
                    RawCallLocation::new(code as usize, 0, lasti as i32)
                }
            }
            _ => RawCallLocation::new(0, 0, 0),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Symbol {
    pub name: String,
    pub file: String,
    pub line: i32,
}

impl<T> TryFrom<*const T> for Symbol
where
    T: CodeObject,
{
    type Error = std::io::Error;

    fn try_from(value: *const T) -> Result<Self, Self::Error> {
        if value.is_null() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Null pointer cannot be converted to Symbol",
            ));
        }
        unsafe {
            let name = (*value).name();
            let file = (*value).filename();
            let line = (*value).first_lineno();

            Ok(Symbol {
                name: copy_string(
                    (*name).address(name as usize) as *const u8,
                    (*name).size() * (*name).kind() as usize,
                    (*name).kind(),
                    (*name).ascii(),
                ),
                file: copy_string(
                    (*file).address(file as usize) as *const u8,
                    (*file).size() * (*file).kind() as usize,
                    (*file).kind(),
                    (*file).ascii(),
                ),
                line,
            })
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CallLocation {
    pub callee: Symbol,
    pub caller: Option<Symbol>,
    pub lineno: i32,
}

impl CallLocation {
    pub fn new(callee: Symbol, caller: Option<Symbol>, lineno: i32) -> Self {
        CallLocation {
            callee,
            caller,
            lineno,
        }
    }
}

impl TryFrom<&RawCallLocation> for CallLocation {
    type Error = std::io::Error;
    fn try_from(value: &RawCallLocation) -> Result<Self, Self::Error> {
        let call_location = match unsafe { (PYVERSION.major, PYVERSION.minor) } {
            (3, 4) | (3, 5) | (3, 6) | (3, 7) | (3, 8) | (3, 9) | (3, 10) => {
                let callee: Symbol =
                    (value.callee as *const python_bindings::v3_10_0::PyCodeObject).try_into()?;
                let caller: Option<Symbol> = (value.caller
                    as *const python_bindings::v3_10_0::PyCodeObject)
                    .try_into()
                    .ok();
                let lineno = parse_lineno(
                    value.caller as *const python_bindings::v3_10_0::PyCodeObject,
                    value.offset,
                );
                CallLocation::new(callee, caller, lineno)
            }
            (3, 11) => {
                let callee: Symbol =
                    (value.callee as *const python_bindings::v3_11_0::PyCodeObject).try_into()?;
                let caller: Option<Symbol> = (value.caller
                    as *const python_bindings::v3_11_0::PyCodeObject)
                    .try_into()
                    .ok();
                let lineno = parse_lineno(
                    value.caller as *const python_bindings::v3_11_0::PyCodeObject,
                    value.offset,
                );
                CallLocation::new(callee, caller, lineno)
            }
            (3, 12) => {
                let callee: Symbol =
                    (value.callee as *const python_bindings::v3_12_0::PyCodeObject).try_into()?;
                let caller: Option<Symbol> = (value.caller
                    as *const python_bindings::v3_12_0::PyCodeObject)
                    .try_into()
                    .ok();
                let lineno = parse_lineno(
                    value.caller as *const python_bindings::v3_12_0::PyCodeObject,
                    value.offset,
                );
                CallLocation::new(callee, caller, lineno)
            }
            (3, 13) => {
                let callee: Symbol =
                    (value.callee as *const python_bindings::v3_13_0::PyCodeObject).try_into()?;
                let caller: Option<Symbol> = (value.caller
                    as *const python_bindings::v3_13_0::PyCodeObject)
                    .try_into()
                    .ok();
                let lineno = parse_lineno(
                    value.caller as *const python_bindings::v3_13_0::PyCodeObject,
                    value.offset,
                );
                CallLocation::new(callee, caller, lineno)
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Unsupported Python version",
                ))
            }
        };
        Ok(call_location)
    }
}

fn parse_lineno<T: CodeObject>(code: *const T, lasti: i32) -> i32 {
    if code.is_null() {
        return 0;
    }
    unsafe {
        let line_table_ptr = (*code).line_table();
        let line_table_size = (*line_table_ptr).size();

        let mut line_table_bytes: Vec<u8> = Vec::with_capacity(line_table_size);
        std::ptr::copy_nonoverlapping(
            line_table_ptr as *const _,
            line_table_bytes.as_mut_ptr(),
            line_table_size,
        );
        line_table_bytes.set_len(line_table_size);
        (*code).get_line_number(lasti, line_table_bytes.as_slice())
    }
}

fn copy_string(addr: *const u8, len: usize, kind: u32, ascii: bool) -> String {
    let len = if len > 1024 { 1024 } else { len };
    match (kind, ascii) {
        (4, _) => {
            let chars = unsafe { std::slice::from_raw_parts(addr as *const char, len / 4) };
            chars.iter().collect()
        }
        (2, _) => {
            let chars = unsafe { std::slice::from_raw_parts(addr as *const u16, len / 2) };
            String::from_utf16(chars).unwrap_or_default()
        }
        (1, true) => {
            let slice = unsafe { std::slice::from_raw_parts(addr, len) };
            String::from_utf8_lossy(slice).to_string()
        }
        (1, false) => {
            let slice = unsafe { std::slice::from_raw_parts(addr, len) };
            String::from_utf8_lossy(slice).to_string()
        }
        _ => String::new(),
    }
}

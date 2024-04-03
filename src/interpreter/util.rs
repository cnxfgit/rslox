pub static mut had_error: bool = false;

pub fn error(line: usize, message: &'static str) {
    report(line, "", message)
}

pub fn report(line: usize, where_: &'static str, message: &'static str) {
    eprintln!("[line {}] Error {}: {}", line, where_, message);
    unsafe {
        had_error = true;
    }
}

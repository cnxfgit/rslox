static mut had_error: bool = false;

pub fn had_error_set(value: bool) {
    unsafe {
        had_error = value;
    }
}

pub fn had_error_get() -> bool {
    unsafe { had_error }
}

pub fn error(line: usize, message: &'static str) {
    report(line, "", message)
}

pub fn report(line: usize, where_: &'static str, message: &'static str) {
    eprintln!("[line {}] Error {}: {}", line, where_, message);
    had_error_set(true);
}

pub fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

pub fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}
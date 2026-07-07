//! Driver cho bàn phím PS/2
//!
//! Giải mã Scancode Set 1 thành KeyEvent và lưu trữ trong bộ đệm an toàn thông qua Spinlock Mutex.

use spin::Mutex;

/// Các mã phím thông dụng được hỗ trợ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Unknown,
    Esc,
    Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0,
    Minus, Equal, Backspace,
    Tab, Q, W, E, R, T, Y, U, I, O, P, LBracket, RBracket, Enter,
    Control, A, S, D, F, G, H, J, K, L, Semicolon, Quote, Backtick,
    LShift, Backslash, Z, X, C, V, B, N, M, Comma, Period, Slash, RShift,
    Alt, Space, CapsLock,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
}

/// Sự kiện phím (nhấn hoặc nhả)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub key_code: KeyCode,
    pub pressed: bool,
}

const BUFFER_SIZE: usize = 128;

/// Bộ đệm bàn phím tĩnh dạng Ring Buffer có giới hạn
struct KeyboardBuffer {
    data: [Option<KeyEvent>; BUFFER_SIZE],
    head: usize,
    tail: usize,
}

static KEYBOARD_BUFFER: Mutex<KeyboardBuffer> = Mutex::new(KeyboardBuffer {
    data: [None; BUFFER_SIZE],
    head: 0,
    tail: 0,
});

/// Trình xử lý scancode nhận được từ interrupt handler bàn phím
pub fn handle_scancode(scancode: u8) {
    let pressed = (scancode & 0x80) == 0;
    let raw_code = scancode & 0x7F;

    let key_code = match raw_code {
        0x01 => KeyCode::Esc,
        0x02 => KeyCode::Key1,
        0x03 => KeyCode::Key2,
        0x04 => KeyCode::Key3,
        0x05 => KeyCode::Key4,
        0x06 => KeyCode::Key5,
        0x07 => KeyCode::Key6,
        0x08 => KeyCode::Key7,
        0x09 => KeyCode::Key8,
        0x0A => KeyCode::Key9,
        0x0B => KeyCode::Key0,
        0x0C => KeyCode::Minus,
        0x0D => KeyCode::Equal,
        0x0E => KeyCode::Backspace,
        0x0F => KeyCode::Tab,
        0x10 => KeyCode::Q,
        0x11 => KeyCode::W,
        0x12 => KeyCode::E,
        0x13 => KeyCode::R,
        0x14 => KeyCode::T,
        0x15 => KeyCode::Y,
        0x16 => KeyCode::U,
        0x17 => KeyCode::I,
        0x18 => KeyCode::O,
        0x19 => KeyCode::P,
        0x1A => KeyCode::LBracket,
        0x1B => KeyCode::RBracket,
        0x1C => KeyCode::Enter,
        0x1D => KeyCode::Control,
        0x1E => KeyCode::A,
        0x1F => KeyCode::S,
        0x20 => KeyCode::D,
        0x21 => KeyCode::F,
        0x22 => KeyCode::G,
        0x23 => KeyCode::H,
        0x24 => KeyCode::J,
        0x25 => KeyCode::K,
        0x26 => KeyCode::L,
        0x27 => KeyCode::Semicolon,
        0x28 => KeyCode::Quote,
        0x29 => KeyCode::Backtick,
        0x2A => KeyCode::LShift,
        0x2B => KeyCode::Backslash,
        0x2C => KeyCode::Z,
        0x2D => KeyCode::X,
        0x2E => KeyCode::C,
        0x2F => KeyCode::V,
        0x30 => KeyCode::B,
        0x31 => KeyCode::N,
        0x32 => KeyCode::M,
        0x33 => KeyCode::Comma,
        0x34 => KeyCode::Period,
        0x35 => KeyCode::Slash,
        0x36 => KeyCode::RShift,
        0x38 => KeyCode::Alt,
        0x39 => KeyCode::Space,
        0x3A => KeyCode::CapsLock,
        0x3B => KeyCode::F1,
        0x3C => KeyCode::F2,
        0x3D => KeyCode::F3,
        0x3E => KeyCode::F4,
        0x3F => KeyCode::F5,
        0x40 => KeyCode::F6,
        0x41 => KeyCode::F7,
        0x42 => KeyCode::F8,
        0x43 => KeyCode::F9,
        0x44 => KeyCode::F10,
        0x57 => KeyCode::F11,
        0x58 => KeyCode::F12,
        _ => KeyCode::Unknown,
    };

    if key_code != KeyCode::Unknown {
        let event = KeyEvent { key_code, pressed };
        let mut buffer = KEYBOARD_BUFFER.lock();
        let next_tail = (buffer.tail + 1) % BUFFER_SIZE;

        if next_tail != buffer.head {
            let tail = buffer.tail;
            buffer.data[tail] = Some(event);
            buffer.tail = next_tail;
        }
        // Nếu buffer đầy, áp dụng chính sách drop event mới
    }
}

/// Thăm dò sự kiện phím tiếp theo trong bộ đệm (dành cho main loop)
pub fn poll_key_event() -> Option<KeyEvent> {
    let mut buffer = KEYBOARD_BUFFER.lock();
    if buffer.head != buffer.tail {
        let event = buffer.data[buffer.head];
        buffer.head = (buffer.head + 1) % BUFFER_SIZE;
        event
    } else {
        None
    }
}

/// Dịch chuyển KeyCode sang ký tự hiển thị tối giản (hỗ trợ phím thường và shift cơ bản)
pub fn keycode_to_char(key_code: KeyCode, shift: bool) -> Option<char> {
    let c = match key_code {
        KeyCode::Space => ' ',
        KeyCode::Key1 => if shift { '!' } else { '1' },
        KeyCode::Key2 => if shift { '@' } else { '2' },
        KeyCode::Key3 => if shift { '#' } else { '3' },
        KeyCode::Key4 => if shift { '$' } else { '4' },
        KeyCode::Key5 => if shift { '%' } else { '5' },
        KeyCode::Key6 => if shift { '^' } else { '6' },
        KeyCode::Key7 => if shift { '&' } else { '7' },
        KeyCode::Key8 => if shift { '*' } else { '8' },
        KeyCode::Key9 => if shift { '(' } else { '9' },
        KeyCode::Key0 => if shift { ')' } else { '0' },
        KeyCode::Minus => if shift { '_' } else { '-' },
        KeyCode::Equal => if shift { '+' } else { '=' },
        KeyCode::LBracket => if shift { '{' } else { '[' },
        KeyCode::RBracket => if shift { '}' } else { ']' },
        KeyCode::Semicolon => if shift { ':' } else { ';' },
        KeyCode::Quote => if shift { '"' } else { '\'' },
        KeyCode::Backtick => if shift { '~' } else { '`' },
        KeyCode::Backslash => if shift { '|' } else { '\\' },
        KeyCode::Comma => if shift { '<' } else { ',' },
        KeyCode::Period => if shift { '>' } else { '.' },
        KeyCode::Slash => if shift { '?' } else { '/' },
        KeyCode::A => if shift { 'A' } else { 'a' },
        KeyCode::B => if shift { 'B' } else { 'b' },
        KeyCode::C => if shift { 'C' } else { 'c' },
        KeyCode::D => if shift { 'D' } else { 'd' },
        KeyCode::E => if shift { 'E' } else { 'e' },
        KeyCode::F => if shift { 'F' } else { 'f' },
        KeyCode::G => if shift { 'G' } else { 'g' },
        KeyCode::H => if shift { 'H' } else { 'h' },
        KeyCode::I => if shift { 'I' } else { 'i' },
        KeyCode::J => if shift { 'J' } else { 'j' },
        KeyCode::K => if shift { 'K' } else { 'k' },
        KeyCode::L => if shift { 'L' } else { 'l' },
        KeyCode::M => if shift { 'M' } else { 'm' },
        KeyCode::N => if shift { 'N' } else { 'n' },
        KeyCode::O => if shift { 'O' } else { 'o' },
        KeyCode::P => if shift { 'P' } else { 'p' },
        KeyCode::Q => if shift { 'Q' } else { 'q' },
        KeyCode::R => if shift { 'R' } else { 'r' },
        KeyCode::S => if shift { 'S' } else { 's' },
        KeyCode::T => if shift { 'T' } else { 't' },
        KeyCode::U => if shift { 'U' } else { 'u' },
        KeyCode::V => if shift { 'V' } else { 'v' },
        KeyCode::W => if shift { 'W' } else { 'w' },
        KeyCode::X => if shift { 'X' } else { 'x' },
        KeyCode::Y => if shift { 'Y' } else { 'y' },
        KeyCode::Z => if shift { 'Z' } else { 'z' },
        _ => return None,
    };
    Some(c)
}

use core::fmt::{self, Write};

use spin::Mutex;

const FONT_WIDTH: usize = 5;
const FONT_HEIGHT: usize = 7;
const GLYPH_SCALE: usize = 2;
const CELL_WIDTH: usize = (FONT_WIDTH + 1) * GLYPH_SCALE;
const CELL_HEIGHT: usize = (FONT_HEIGHT + 2) * GLYPH_SCALE;

static FRAMEBUFFER_CONSOLE: Mutex<Option<TextConsole>> = Mutex::new(None);

#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    pub address: usize,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bytes_per_pixel: usize,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}

#[derive(Clone, Copy)]
pub enum ConsoleError {
    Unavailable,
    InvalidGeometry,
    UnsupportedPixelFormat,
}

impl ConsoleError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "framebuffer unavailable",
            Self::InvalidGeometry => "invalid framebuffer geometry",
            Self::UnsupportedPixelFormat => "unsupported framebuffer pixel format",
        }
    }
}

#[derive(Clone, Copy)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

struct TextConsole {
    info: FramebufferInfo,
    frame_size: usize,
    cursor_x: usize,
    cursor_y: usize,
    columns: usize,
    rows: usize,
    foreground: Color,
    background: Color,
}

/// Khởi tạo framebuffer console từ metadata do Limine cung cấp.
pub fn init_framebuffer_console(info: FramebufferInfo) -> Result<(), ConsoleError> {
    if info.address == 0 {
        return Err(ConsoleError::Unavailable);
    }

    if info.width == 0 || info.height == 0 || info.pitch == 0 {
        return Err(ConsoleError::InvalidGeometry);
    }

    if info.memory_model != 1 || !(info.bytes_per_pixel == 3 || info.bytes_per_pixel == 4) {
        return Err(ConsoleError::UnsupportedPixelFormat);
    }

    let Some(frame_size) = info.pitch.checked_mul(info.height) else {
        return Err(ConsoleError::InvalidGeometry);
    };

    let columns = info.width / CELL_WIDTH;
    let rows = info.height / CELL_HEIGHT;
    if columns == 0 || rows == 0 {
        return Err(ConsoleError::InvalidGeometry);
    }

    let mut console = TextConsole {
        info,
        frame_size,
        cursor_x: 0,
        cursor_y: 0,
        columns,
        rows,
        foreground: Color {
            red: 220,
            green: 235,
            blue: 255,
        },
        background: Color {
            red: 0,
            green: 0,
            blue: 0,
        },
    };
    console.clear();
    *FRAMEBUFFER_CONSOLE.lock() = Some(console);

    Ok(())
}

/// Ghi dữ liệu định dạng ra framebuffer nếu console đã sẵn sàng.
pub fn framebuffer_print(args: fmt::Arguments) {
    if let Some(console) = FRAMEBUFFER_CONSOLE.lock().as_mut() {
        let _ = console.write_fmt(args);
    }
}

/// Ghi dữ liệu định dạng ra framebuffer kèm xuống dòng nếu console đã sẵn sàng.
pub fn framebuffer_println(args: fmt::Arguments) {
    if let Some(console) = FRAMEBUFFER_CONSOLE.lock().as_mut() {
        let _ = console.write_fmt(args);
        console.newline();
    }
}

impl TextConsole {
    fn clear(&mut self) {
        for y in 0..self.info.height {
            for x in 0..self.info.width {
                self.write_pixel(x, y, self.background);
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;

        if self.cursor_y >= self.rows {
            self.clear();
        }
    }

    fn put_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.cursor_x = 0,
            byte => {
                if self.cursor_x >= self.columns {
                    self.newline();
                }

                let x = self.cursor_x * CELL_WIDTH;
                let y = self.cursor_y * CELL_HEIGHT;
                self.draw_cell_background(x, y);
                self.draw_glyph(x, y + GLYPH_SCALE, glyph_for(byte));
                self.cursor_x += 1;
            }
        }
    }

    fn draw_cell_background(&mut self, base_x: usize, base_y: usize) {
        for y in 0..CELL_HEIGHT {
            for x in 0..CELL_WIDTH {
                self.write_pixel(base_x + x, base_y + y, self.background);
            }
        }
    }

    fn draw_glyph(&mut self, base_x: usize, base_y: usize, glyph: [u8; FONT_HEIGHT]) {
        for (row_index, row) in glyph.iter().copied().enumerate() {
            for column in 0..FONT_WIDTH {
                let bit = (row >> (FONT_WIDTH - 1 - column)) & 1;
                let color = if bit == 1 {
                    self.foreground
                } else {
                    self.background
                };

                for scale_y in 0..GLYPH_SCALE {
                    for scale_x in 0..GLYPH_SCALE {
                        self.write_pixel(
                            base_x + (column * GLYPH_SCALE) + scale_x,
                            base_y + (row_index * GLYPH_SCALE) + scale_y,
                            color,
                        );
                    }
                }
            }
        }
    }

    fn write_pixel(&self, x: usize, y: usize, color: Color) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }

        let Some(row_offset) = y.checked_mul(self.info.pitch) else {
            return;
        };
        let Some(column_offset) = x.checked_mul(self.info.bytes_per_pixel) else {
            return;
        };
        let Some(offset) = row_offset.checked_add(column_offset) else {
            return;
        };
        let Some(end) = offset.checked_add(self.info.bytes_per_pixel) else {
            return;
        };
        if end > self.frame_size {
            return;
        }

        let encoded = self.encode_color(color);
        let pixel = (self.info.address + offset) as *mut u8;

        // SAFETY: Offset và số byte ghi đã được kiểm tra nằm trong framebuffer;
        // write_volatile phù hợp cho vùng memory-mapped framebuffer.
        unsafe {
            for byte_index in 0..self.info.bytes_per_pixel {
                pixel
                    .add(byte_index)
                    .write_volatile(((encoded >> (byte_index * 8)) & 0xff) as u8);
            }
        }
    }

    fn encode_color(&self, color: Color) -> u32 {
        (scale_component(color.red, self.info.red_mask_size) << self.info.red_mask_shift)
            | (scale_component(color.green, self.info.green_mask_size)
                << self.info.green_mask_shift)
            | (scale_component(color.blue, self.info.blue_mask_size) << self.info.blue_mask_shift)
    }
}

impl Write for TextConsole {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            self.put_byte(byte);
        }

        Ok(())
    }
}

fn scale_component(value: u8, bits: u8) -> u32 {
    if bits == 0 {
        return 0;
    }

    if bits >= 8 {
        return u32::from(value);
    }

    let max = (1u32 << bits) - 1;
    (u32::from(value) * max + 127) / 255
}

fn glyph_for(byte: u8) -> [u8; FONT_HEIGHT] {
    match byte {
        b' ' => [0, 0, 0, 0, 0, 0, 0],
        b'[' => [
            0b11110, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11110,
        ],
        b']' => [
            0b01111, 0b00001, 0b00001, 0b00001, 0b00001, 0b00001, 0b01111,
        ],
        b':' => [0, 0b00100, 0b00100, 0, 0b00100, 0b00100, 0],
        b'-' => [0, 0, 0, 0b11111, 0, 0, 0],
        b'0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        b'1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        b'2' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        b'3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        b'5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        b'6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        b'7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        b'8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        b'9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        b'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        b'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        b'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        b'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        b'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        b'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        b'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        b'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        b'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        b'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        b'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        b'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        b'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        b'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        b'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        b'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        b'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        b'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        b'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        b'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        b'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100,
        ],
        b'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        b'X' => [
            0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b01010, 0b10001,
        ],
        b'Y' => [
            0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        b'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        b'a' => [0, 0b01110, 0b00001, 0b01111, 0b10001, 0b10011, 0b01101],
        b'b' => [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110,
        ],
        b'c' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110],
        b'd' => [
            0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111,
        ],
        b'e' => [0, 0b01110, 0b10001, 0b11111, 0b10000, 0b10001, 0b01110],
        b'f' => [
            0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000,
        ],
        b'g' => [0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        b'h' => [
            0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001,
        ],
        b'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'j' => [0b00010, 0, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100],
        b'k' => [
            0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010,
        ],
        b'l' => [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        b'm' => [0, 0, 0b11010, 0b10101, 0b10101, 0b10101, 0b10101],
        b'n' => [0, 0, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001],
        b'o' => [0, 0, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110],
        b'p' => [0, 0, 0b11110, 0b10001, 0b11110, 0b10000, 0b10000],
        b'q' => [0, 0, 0b01111, 0b10001, 0b01111, 0b00001, 0b00001],
        b'r' => [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000],
        b's' => [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        b't' => [
            0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110,
        ],
        b'u' => [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        b'v' => [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        b'w' => [0, 0, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010],
        b'x' => [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        b'y' => [0, 0, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        b'z' => [0, 0, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111],
        _ => [0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0, 0b00100],
    }
}

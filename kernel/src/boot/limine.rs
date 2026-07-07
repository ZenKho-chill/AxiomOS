use core::{cell::UnsafeCell, ptr::null};

use crate::console::framebuffer::FramebufferInfo;

#[repr(C)]
struct LimineBaseRevision {
    _magic: UnsafeCell<[u64; 3]>,
}

// SAFETY: Limine có thể cập nhật request tĩnh trước khi chuyển quyền cho kernel;
// sau `_start`, Rust chỉ giữ object này để bootloader quét và không mutation song song.
unsafe impl Sync for LimineBaseRevision {}

impl LimineBaseRevision {
    const fn with_revision(revision: u64) -> Self {
        Self {
            _magic: UnsafeCell::new([0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, revision]),
        }
    }
}

#[repr(C)]
struct LimineFramebufferRequest {
    _magic: [u64; 2],
    _id: [u64; 2],
    _revision: u64,
    response: UnsafeCell<*const LimineFramebufferResponse>,
}

// SAFETY: Field response chỉ được Limine ghi trong giai đoạn boot handoff;
// kernel đọc bằng volatile sau khi bootloader đã chuyển quyền và không ghi lại field này.
unsafe impl Sync for LimineFramebufferRequest {}

impl LimineFramebufferRequest {
    const fn new() -> Self {
        Self {
            _magic: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b],
            _id: [0x9d5827dcd881dd75, 0xa3148604f6fab11b],
            _revision: 0,
            response: UnsafeCell::new(null()),
        }
    }

    fn response(&self) -> Option<&'static LimineFramebufferResponse> {
        // SAFETY: Limine ghi con trỏ response vào field này trước khi gọi `_start`;
        // đọc volatile tránh compiler cache giá trị null ban đầu.
        let response = unsafe { self.response.get().read_volatile() };
        if response.is_null() {
            return None;
        }

        // SAFETY: Con trỏ không null đến response thuộc quyền sở hữu bootloader
        // và còn hợp lệ trong suốt thời gian kernel chạy ở giai đoạn boot sớm.
        Some(unsafe { &*response })
    }
}

#[repr(C)]
struct LimineFramebufferResponse {
    _revision: u64,
    framebuffer_count: u64,
    framebuffers: *const *const LimineFramebuffer,
}

#[repr(C)]
struct LimineFramebuffer {
    address: *mut u8,
    width: u64,
    height: u64,
    pitch: u64,
    bpp: u16,
    memory_model: u8,
    red_mask_size: u8,
    red_mask_shift: u8,
    green_mask_size: u8,
    green_mask_shift: u8,
    blue_mask_size: u8,
    blue_mask_shift: u8,
    _reserved0: [u8; 7],
    _edid_size: u64,
    _edid: *const u8,
}

#[repr(C)]
struct LimineHhdmRequest {
    _magic: [u64; 2],
    _id: [u64; 2],
    _revision: u64,
    response: UnsafeCell<*const LimineHhdmResponse>,
}

// SAFETY: Limine chỉ ghi field response trong giai đoạn boot handoff trước `_start`;
// sau đó kernel chỉ đọc volatile và không ghi lại request tĩnh này.
unsafe impl Sync for LimineHhdmRequest {}

impl LimineHhdmRequest {
    const fn new() -> Self {
        Self {
            _magic: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b],
            _id: [0x48dcf1cb8ad2b852, 0x63984e959a98244b],
            _revision: 0,
            response: UnsafeCell::new(null()),
        }
    }

    fn response(&self) -> Option<&'static LimineHhdmResponse> {
        // SAFETY: Limine cập nhật con trỏ response trước khi trao quyền cho kernel;
        // đọc volatile tránh compiler giữ giá trị null ban đầu.
        let response = unsafe { self.response.get().read_volatile() };
        if response.is_null() {
            return None;
        }
        // SAFETY: Con trỏ response không null và trỏ tới cấu trúc do Limine giữ sống
        // trong giai đoạn kernel boot sớm.
        Some(unsafe { &*response })
    }
}

#[repr(C)]
struct LimineHhdmResponse {
    _revision: u64,
    offset: u64,
}

#[repr(C)]
struct LimineMemoryMapRequest {
    _magic: [u64; 2],
    _id: [u64; 2],
    _revision: u64,
    response: UnsafeCell<*const LimineMemoryMapResponse>,
}

// SAFETY: Limine chỉ ghi field response trong giai đoạn boot handoff trước `_start`;
// kernel giữ request tĩnh để đọc volatile và không mutation song song.
unsafe impl Sync for LimineMemoryMapRequest {}

impl LimineMemoryMapRequest {
    const fn new() -> Self {
        Self {
            _magic: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b],
            _id: [0x67cf3d9d378a806f, 0xe304acdfc50c3c62],
            _revision: 0,
            response: UnsafeCell::new(null()),
        }
    }

    fn response(&self) -> Option<&'static LimineMemoryMapResponse> {
        // SAFETY: Limine cập nhật con trỏ response trước khi trao quyền cho kernel;
        // đọc volatile tránh compiler giữ giá trị null ban đầu.
        let response = unsafe { self.response.get().read_volatile() };
        if response.is_null() {
            return None;
        }
        // SAFETY: Con trỏ response không null và trỏ tới memory-map response
        // do Limine giữ sống trong giai đoạn boot sớm.
        Some(unsafe { &*response })
    }
}

#[repr(C)]
struct LimineMemoryMapResponse {
    _revision: u64,
    entry_count: u64,
    entries: *const *const LimineMmapEntry,
}

#[repr(C)]
#[derive(Debug)]
pub struct LimineMmapEntry {
    pub base: u64,
    pub length: u64,
    pub entry_type: u64,
}

#[used]
#[link_section = ".requests_start_marker"]
static REQUESTS_START_MARKER: [u64; 4] = [
    0xf6b8f4b39de7d1ae,
    0xfab91a6940fcb9cf,
    0x785c6ed015d3e316,
    0x181e920a7852b9d9,
];

#[used]
#[link_section = ".requests"]
static BASE_REVISION: LimineBaseRevision = LimineBaseRevision::with_revision(3);

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: LimineFramebufferRequest = LimineFramebufferRequest::new();

#[used]
#[link_section = ".requests"]
static HHDM_REQUEST: LimineHhdmRequest = LimineHhdmRequest::new();

#[used]
#[link_section = ".requests"]
static MEMORY_MAP_REQUEST: LimineMemoryMapRequest = LimineMemoryMapRequest::new();

#[used]
#[link_section = ".requests_end_marker"]
static REQUESTS_END_MARKER: [u64; 2] = [0xadc0e0531bb10d03, 0x9572709f31764c62];

/// Giữ các request Limine sống trong binary để bootloader có thể quét thấy.
pub fn keep_requests_alive() {
    let _ = &REQUESTS_START_MARKER;
    let _ = &BASE_REVISION;
    let _ = &FRAMEBUFFER_REQUEST;
    let _ = &HHDM_REQUEST;
    let _ = &MEMORY_MAP_REQUEST;
    let _ = &REQUESTS_END_MARKER;
}

/// Lấy framebuffer đầu tiên do Limine bàn giao nếu có.
pub fn framebuffer_info() -> Option<FramebufferInfo> {
    let response = FRAMEBUFFER_REQUEST.response()?;
    if response.framebuffer_count == 0 || response.framebuffers.is_null() {
        return None;
    }

    // SAFETY: `framebuffers` trỏ tới mảng con trỏ framebuffer do Limine cung cấp;
    // count > 0 đã được kiểm tra nên đọc phần tử đầu là hợp lệ nếu con trỏ không null.
    let framebuffer = unsafe { response.framebuffers.read_volatile() };
    if framebuffer.is_null() {
        return None;
    }

    // SAFETY: Con trỏ framebuffer không null và metadata framebuffer thuộc quyền
    // sở hữu bootloader trong giai đoạn boot sớm.
    let framebuffer = unsafe { &*framebuffer };

    Some(FramebufferInfo {
        address: framebuffer.address as usize,
        width: framebuffer.width as usize,
        height: framebuffer.height as usize,
        pitch: framebuffer.pitch as usize,
        bytes_per_pixel: (usize::from(framebuffer.bpp) + 7) / 8,
        memory_model: framebuffer.memory_model,
        red_mask_size: framebuffer.red_mask_size,
        red_mask_shift: framebuffer.red_mask_shift,
        green_mask_size: framebuffer.green_mask_size,
        green_mask_shift: framebuffer.green_mask_shift,
        blue_mask_size: framebuffer.blue_mask_size,
        blue_mask_shift: framebuffer.blue_mask_shift,
    })
}

/// Lấy HHDM offset do Limine bàn giao.
pub fn hhdm_offset() -> Option<u64> {
    let response = HHDM_REQUEST.response()?;
    Some(response.offset)
}

/// Trả về một slice raw pointer các Memory Map Entry do Limine cung cấp.
pub fn memory_map_raw() -> Option<(*const *const LimineMmapEntry, usize)> {
    let response = MEMORY_MAP_REQUEST.response()?;
    if response.entry_count == 0 || response.entries.is_null() {
        return None;
    }
    Some((response.entries, response.entry_count as usize))
}

//! Trình phân tích định dạng tệp tin ELF64 (ELF64 Parser)
//!
//! Mô-đun này cung cấp các cấu trúc dữ liệu và hàm phân tích cú pháp an toàn
//! cho định dạng tệp tin ELF64 dành cho kiến trúc x86_64.

use alloc::vec::Vec;

/// Các lỗi có thể xảy ra trong quá trình phân tích tệp tin ELF
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// Magic number không hợp lệ (không phải \x7fELF)
    InvalidMagic,
    /// Định dạng không phải 64-bit
    UnsupportedClass,
    /// Thứ tự byte không phải Little Endian
    UnsupportedEndian,
    /// Phiên bản ELF không hợp lệ
    InvalidVersion,
    /// Kiến trúc máy tính không phải x86_64
    UnsupportedMachine,
    /// Kích thước ELF Header không khớp
    InvalidHeaderSize,
    /// Kích thước Program Header Entry không khớp
    InvalidProgramHeaderSize,
    /// Chỉ số vượt ra ngoài phạm vi dữ liệu tệp tin
    OutOfBounds,
    /// Tràn số nguyên khi tính toán offsets
    IntegerOverflow,
}

impl ElfError {
    /// Trả về mô tả lỗi dạng chuỗi tĩnh
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidMagic => "Magic number không hợp lệ",
            Self::UnsupportedClass => "Định dạng không phải 64-bit (chỉ hỗ trợ ELF64)",
            Self::UnsupportedEndian => "Chỉ hỗ trợ Little Endian",
            Self::InvalidVersion => "Phiên bản ELF không hợp lệ",
            Self::UnsupportedMachine => "Kiến trúc không hỗ trợ (yêu cầu x86_64)",
            Self::InvalidHeaderSize => "Kích thước ELF Header không hợp lệ",
            Self::InvalidProgramHeaderSize => "Kích thước Program Header Entry không hợp lệ",
            Self::OutOfBounds => "Offset vượt quá kích thước tệp tin",
            Self::IntegerOverflow => "Tràn số nguyên trong quá trình tính toán offset",
        }
    }
}

/// ELF Header của định dạng ELF64 (64 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader64 {
    /// Định danh ELF (Magic, Class, Endian, v.v.)
    pub e_ident: [u8; 16],
    /// Loại tệp đối tượng (Executable, Relocatable, v.v.)
    pub e_type: u16,
    /// Kiến trúc máy tính đích (x86_64 là 0x3E)
    pub e_machine: u16,
    /// Phiên bản ELF
    pub e_version: u32,
    /// Địa chỉ ảo điểm vào chương trình (Entry point)
    pub e_entry: u64,
    /// Offset của bảng Program Header Table trong file
    pub e_phoff: u64,
    /// Offset của bảng Section Header Table trong file
    pub e_shoff: u64,
    /// Các cờ đặc thù cho kiến trúc bộ xử lý
    pub e_flags: u32,
    /// Kích thước của ELF Header (thường là 64 bytes)
    pub e_ehsize: u16,
    /// Kích thước của mỗi entry trong Program Header Table (thường là 56 bytes)
    pub e_phentsize: u16,
    /// Số lượng entries trong Program Header Table
    pub e_phnum: u16,
    /// Kích thước của mỗi entry trong Section Header Table
    pub e_shentsize: u16,
    /// Số lượng entries trong Section Header Table
    pub e_shnum: u16,
    /// Chỉ số của Section Header chứa tên các phân đoạn
    pub e_shstrndx: u16,
}

/// Định nghĩa một Program Header trong định dạng ELF64 (56 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramHeader64 {
    /// Loại segment (1 = PT_LOAD, v.v.)
    pub p_type: u32,
    /// Quyền truy cập phân đoạn (cờ: 1=X, 2=W, 4=R)
    pub p_flags: u32,
    /// Offset của segment trong file
    pub p_offset: u64,
    /// Địa chỉ ảo của segment trong bộ nhớ
    pub p_vaddr: u64,
    /// Địa chỉ vật lý của segment (bị bỏ qua)
    pub p_paddr: u64,
    /// Kích thước của segment trong file
    pub p_filesz: u64,
    /// Kích thước của segment trong bộ nhớ ảo
    pub p_memsz: u64,
    /// Căn lề của segment (thường là lũy thừa của 2, như 4096)
    pub p_align: u64,
}

/// Metadata trích xuất từ ELF Header sau khi validate thành công
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfMetadata {
    /// Địa chỉ điểm vào chương trình
    pub entry: u64,
    /// Offset bảng Program Header Table
    pub ph_offset: u64,
    /// Số lượng Program Headers
    pub ph_count: u16,
    /// Kích thước mỗi Program Header entry
    pub ph_entry_size: u16,
}

/// Kiểm tra tính hợp lệ và trích xuất thông tin cơ bản từ ELF64 Header
pub fn validate_elf64(bytes: &[u8]) -> Result<ElfMetadata, ElfError> {
    if bytes.len() < 64 {
        return Err(ElfError::OutOfBounds);
    }

    // Đọc an toàn thông qua sao chép byte thô để tránh alignment panic trên một số kiến trúc
    // SAFETY: Chúng ta đã kiểm tra bounds và kích thước của ElfHeader64 đúng bằng 64 bytes.
    let header = unsafe {
        let mut h = core::mem::MaybeUninit::<ElfHeader64>::uninit();
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), h.as_mut_ptr() as *mut u8, 64);
        h.assume_init()
    };

    // Kiểm tra Magic number: 0x7F 'E' 'L' 'F'
    if header.e_ident[0..4] != [0x7F, b'E', b'L', b'F'] {
        return Err(ElfError::InvalidMagic);
    }

    // Kiểm tra Class: phải là 2 (64-bit)
    if header.e_ident[4] != 2 {
        return Err(ElfError::UnsupportedClass);
    }

    // Kiểm tra Endianness: phải là 1 (Little Endian)
    if header.e_ident[5] != 1 {
        return Err(ElfError::UnsupportedEndian);
    }

    // Kiểm tra ELF version: phải là 1
    if header.e_ident[6] != 1 || header.e_version != 1 {
        return Err(ElfError::InvalidVersion);
    }

    // Kiểm tra kiến trúc CPU: phải là 0x3E (x86_64)
    if header.e_machine != 0x3E {
        return Err(ElfError::UnsupportedMachine);
    }

    // Kiểm tra kích thước header thực tế
    if header.e_ehsize < 64 {
        return Err(ElfError::InvalidHeaderSize);
    }

    // Kiểm tra kích thước của Program Header entry (nếu có program headers)
    if header.e_phnum > 0 && header.e_phentsize != 56 {
        return Err(ElfError::InvalidProgramHeaderSize);
    }

    Ok(ElfMetadata {
        entry: header.e_entry,
        ph_offset: header.e_phoff,
        ph_count: header.e_phnum,
        ph_entry_size: header.e_phentsize,
    })
}

/// Phân tích bảng Program Headers từ tệp tin ELF
pub fn parse_program_headers(
    bytes: &[u8],
    ph_offset: u64,
    ph_count: u16,
    ph_entry_size: u16,
) -> Result<Vec<ProgramHeader64>, ElfError> {
    if ph_count == 0 {
        return Ok(Vec::new());
    }

    if ph_entry_size != 56 {
        return Err(ElfError::InvalidProgramHeaderSize);
    }

    let mut headers = Vec::with_capacity(ph_count as usize);

    for i in 0..ph_count {
        // Tính toán offset an toàn tránh integer overflow
        let offset = (i as u64)
            .checked_mul(ph_entry_size as u64)
            .and_then(|val| val.checked_add(ph_offset))
            .ok_or(ElfError::IntegerOverflow)?;

        let end_offset = offset
            .checked_add(ph_entry_size as u64)
            .ok_or(ElfError::IntegerOverflow)?;

        if end_offset > bytes.len() as u64 {
            return Err(ElfError::OutOfBounds);
        }

        // Đọc an toàn thông qua sao chép byte thô
        // SAFETY: Chúng ta đã kiểm tra bounds và kích thước của ProgramHeader64 đúng bằng 56 bytes.
        let ph = unsafe {
            let mut p = core::mem::MaybeUninit::<ProgramHeader64>::uninit();
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr().add(offset as usize),
                p.as_mut_ptr() as *mut u8,
                56,
            );
            p.assume_init()
        };

        headers.push(ph);
    }

    Ok(headers)
}

/// Chạy chẩn đoán (diagnostics) cho bộ phân tích định dạng ELF64
pub fn run_elf_parser_diagnostics() {
    crate::serial_println!("[AXIOMOS ELF] Chạy chẩn đoán trình phân tích ELF64...");

    // 1. Tạo buffer ELF giả lập hợp lệ
    let mut mock_elf = [0u8; 120];

    // Thiết lập ELF Header (64 bytes)
    mock_elf[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']); // Magic
    mock_elf[4] = 2; // Class (64-bit)
    mock_elf[5] = 1; // Endian (Little Endian)
    mock_elf[6] = 1; // Version
    mock_elf[16..18].copy_from_slice(&2u16.to_le_bytes()); // e_type (Executable)
    mock_elf[18..20].copy_from_slice(&0x3Eu16.to_le_bytes()); // e_machine (x86_64)
    mock_elf[20..24].copy_from_slice(&1u32.to_le_bytes()); // e_version
    mock_elf[24..32].copy_from_slice(&0x401000u64.to_le_bytes()); // e_entry (0x401000)
    mock_elf[32..40].copy_from_slice(&64u64.to_le_bytes()); // e_phoff (64)
    mock_elf[48..52].copy_from_slice(&0u32.to_le_bytes()); // e_flags
    mock_elf[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize (64)
    mock_elf[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize (56)
    mock_elf[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum (1 program header)

    // Thiết lập Program Header giả lập (56 bytes, bắt đầu từ offset 64)
    // p_type (1 = PT_LOAD)
    mock_elf[64..68].copy_from_slice(&1u32.to_le_bytes());
    // p_flags (5 = Read + Execute)
    mock_elf[68..72].copy_from_slice(&5u32.to_le_bytes());
    // p_offset (0)
    mock_elf[72..80].copy_from_slice(&0u64.to_le_bytes());
    // p_vaddr (0x400000)
    mock_elf[80..88].copy_from_slice(&0x400000u64.to_le_bytes());
    // p_paddr (0)
    mock_elf[88..96].copy_from_slice(&0u64.to_le_bytes());
    // p_filesz (120)
    mock_elf[96..104].copy_from_slice(&120u64.to_le_bytes());
    // p_memsz (4096)
    mock_elf[104..112].copy_from_slice(&4096u64.to_le_bytes());
    // p_align (4096)
    mock_elf[112..120].copy_from_slice(&4096u64.to_le_bytes());

    // 2. Kiểm thử validate header hợp lệ
    let meta = validate_elf64(&mock_elf).expect("Lỗi: ELF hợp lệ bị từ chối!");
    assert_eq!(meta.entry, 0x401000);
    assert_eq!(meta.ph_offset, 64);
    assert_eq!(meta.ph_count, 1);
    assert_eq!(meta.ph_entry_size, 56);

    // 3. Kiểm thử parse program headers hợp lệ
    let phs = parse_program_headers(&mock_elf, meta.ph_offset, meta.ph_count, meta.ph_entry_size)
        .expect("Lỗi: Không parse được program headers hợp lệ!");
    assert_eq!(phs.len(), 1);
    assert_eq!(phs[0].p_type, 1); // PT_LOAD
    assert_eq!(phs[0].p_flags, 5); // RX
    assert_eq!(phs[0].p_vaddr, 0x400000);
    assert_eq!(phs[0].p_memsz, 4096);

    // 4. Kiểm thử các trường hợp lỗi

    // Magic number hỏng
    let mut bad_magic = mock_elf;
    bad_magic[0] = 0x00;
    assert_eq!(validate_elf64(&bad_magic), Err(ElfError::InvalidMagic));

    // Sai Machine (không phải x86_64)
    let mut bad_machine = mock_elf;
    bad_machine[18..20].copy_from_slice(&0u16.to_le_bytes());
    assert_eq!(
        validate_elf64(&bad_machine),
        Err(ElfError::UnsupportedMachine)
    );

    // Cỡ header quá nhỏ
    let mut bad_ehsize = mock_elf;
    bad_ehsize[52..54].copy_from_slice(&32u16.to_le_bytes());
    assert_eq!(
        validate_elf64(&bad_ehsize),
        Err(ElfError::InvalidHeaderSize)
    );

    // Vượt bounds dữ liệu tệp tin
    assert_eq!(
        parse_program_headers(
            &mock_elf[0..100],
            meta.ph_offset,
            meta.ph_count,
            meta.ph_entry_size
        ),
        Err(ElfError::OutOfBounds)
    );

    crate::serial_println!("[AXIOMOS ELF] Chạy chẩn đoán trình phân tích ELF64: THÀNH CÔNG");
}

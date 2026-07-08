# ADR-007: Layout Địa chỉ Userspace và Giao diện Cuộc gọi Hệ thống (Syscall ABI)

- **Trạng thái**: APPROVED
- **Người phụ trách**: Kỹ sư trưởng AxiomOS
- **Ngày tạo**: 2026-07-08
- **Ngày cập nhật**: 2026-07-08

---

## Bối cảnh và vấn đề cần giải quyết

Milestone 6 yêu cầu AxiomOS có khả năng nạp chương trình ELF64 từ ổ đĩa ảo (FAT32), tạo lập không gian địa chỉ Ring 3 riêng biệt cho tiến trình và thực thi tiến trình `init` đầu tiên. Để thực hiện điều này, cần có quyết định kiến trúc thống nhất về:
1. **Layout của Không gian địa chỉ người dùng (Userspace Address Space Layout)**: Vị trí nạp mã lệnh, vùng stack và vùng heap để linker của chương trình userspace và trình nạp (loader) của kernel hoạt động khớp nhau.
2. **Cơ chế và Giao diện Cuộc gọi Hệ thống (Syscall ABI)**: Cách thức chương trình Ring 3 yêu cầu dịch vụ từ Ring 0 (kernel), các thanh ghi được sử dụng để truyền tham số và các syscall tối thiểu cần thiết để hỗ trợ lifecycle của tiến trình đầu tiên.

## Các phương án cân nhắc

### 1. Về Layout Địa chỉ Bộ nhớ Userspace
Trong kiến trúc x86_64 canonical address space, userspace chiếm nửa dưới của bộ nhớ (từ `0` đến `0x00007FFFFFFFFFFF`).

- **Phương án A: Địa chỉ ngẫu nhiên hóa (ASLR - Address Space Layout Randomization)**
  - *Ưu điểm*: Bảo mật cao, ngăn chặn khai thác bộ nhớ.
  - *Nhược điểm*: Phức tạp khi triển khai, đòi hỏi kernel loader hỗ trợ relocation động (position-independent executables) và quản lý bộ nhớ phức tạp. Quá sớm cho Milestone 6.
- **Phương án B: Địa chỉ tĩnh cố định (Fixed Static Layout)**
  - *Ưu điểm*: Đơn giản nhất. Linker script của userspace có thể hardcode các địa chỉ cố định. Loader của kernel chỉ cần map trực tiếp các segments của ELF vào đúng địa chỉ chỉ định.
  - *Nhược điểm*: Không bảo mật đối với các hệ thống production (nhưng AxiomOS hiện tại là OS học tập, thử nghiệm).

### 2. Về Cơ chế Syscall (Syscall Mechanism)

- **Phương án A: Sử dụng Ngắt Mềm (Software Interrupt - `int 0x80`)**
  - *Ưu điểm*: Dễ cài đặt, tương tự cơ chế xử lý ngoại lệ hiện có trong IDT của AxiomOS (Spec 005).
  - *Nhược điểm*: Hiệu năng thấp do CPU phải đi qua IDT gate descriptor, kiểm tra phân quyền phức tạp. Không phải là cơ chế tối ưu cho x86_64 Long Mode.
- **Phương án B: Sử dụng lệnh `syscall` / `sysret` của x86_64**
  - *Ưu điểm*: Là cơ chế chuẩn của kiến trúc x86_64, bỏ qua IDT gate và chuyển đổi trực tiếp Ring 0/3 qua các Model Specific Registers (MSRs). Tốc độ cực nhanh.
  - *Nhược điểm*: Đòi hỏi cấu hình MSRs phức tạp và quản lý cẩn thận các thanh ghi `rcx` (lưu `rip` cũ) và `r11` (lưu `rflags` cũ) do CPU tự động ghi đè.

## Quyết định lựa chọn

### 1. Quyết định Layout bộ nhớ Userspace (Fixed Static Layout)
Chúng ta chọn **Phương án B: Địa chỉ tĩnh cố định**.
Layout cụ thể cho một tiến trình userspace như sau:
- **Mã lệnh (Code/Data segments)**: Bắt đầu từ `0x400000` (đây là địa chỉ base mặc định của hầu hết các linker x86_64 bao gồm GNU `ld` và Rust LLD).
- **Ngăn xếp (Userspace Stack)**: Đặt ở vùng nhớ cao của userspace canonical address. Điểm bắt đầu (stack pointer ban đầu `rsp`) sẽ là `0x00007FFFFFFFF000` (được căn lề trang). Kích thước stack ban đầu là 16 KiB (4 trang ảo, từ `0x00007FFFFFFFB000` đến `0x00007FFFFFFFF000`).
- **Heap userspace**: Bắt đầu tại `0x10000000` (256 MiB) và phát triển lên trên. Vùng này sẽ được quản lý bởi syscall mở rộng bộ nhớ ảo sau này.

### 2. Quyết định Cơ chế và Giao diện Syscall (Syscall ABI)
Chúng ta chọn **Phương án B: Sử dụng lệnh `syscall` / `sysret`**.
Cấu hình phần cứng:
- Kích hoạt bit `SCE` (System Call Extension) trong MSR `IA32_EFER` (`0xC0000080`).
- Cấu hình MSR `IA32_STAR` (`0xC0000081`) để định nghĩa Code/Data segment selectors cho Kernel và Userspace. Do yêu cầu phần cứng x86_64, GDT bắt buộc phải sắp xếp các selector theo thứ tự:
  - Kernel Code Selector (GDT index 1, DPL 0)
  - Kernel Data Selector (GDT index 2, DPL 0)
  - User Data Selector (GDT index 3, DPL 3)
  - User Code Selector (GDT index 4, DPL 3)
- Cấu hình MSR `IA32_LSTAR` (`0xC0000082`) trỏ đến địa chỉ của hàm Assembly xử lý syscall entry (`sys_entry`).
- Cấu hình MSR `IA32_FMASK` (`0xC0000084`) để mask các flags khi chuyển đổi (ví dụ: tắt ngắt bằng cách mask flag ngắt IF).

Quy ước truyền tham số (tương thích System V AMD64 ABI):
- **Syscall ID**: Nằm trong thanh ghi `rax`.
- **Các tham số (tối đa 6)**: `rdi` (param 1), `rsi` (param 2), `rdx` (param 3), `r10` (param 4 - lưu ý `rcx` bị ghi đè bởi CPU nên dùng `r10` thay thế), `r8` (param 5), `r9` (param 6).
- **Giá trị trả về**: Nằm trong thanh ghi `rax`. Giá trị âm biểu thị mã lỗi (ví dụ `-1` cho lỗi không xác định).
- **Registers được bảo toàn**: Mọi thanh ghi đa dụng khác (trừ `rax`, `rcx`, `r11`) phải được kernel handler lưu lại và khôi phục trước khi trở lại userspace.

### 3. Danh sách các Syscall đầu tiên
- `sys_exit` (rax = 1): Kết thúc tiến trình hiện tại.
  - Tham số: `rdi` = mã thoát (exit code).
- `sys_write` (rax = 2): Ghi dữ liệu ra thiết bị/file.
  - Tham số: `rdi` = file descriptor (`1` cho stdout/serial), `rsi` = con trỏ trỏ tới buffer bộ nhớ, `rdx` = độ dài buffer.
- `sys_yield` (rax = 3): Tự nguyện nhường CPU cho tiến trình khác.
  - Không có tham số.

## Hệ quả và ảnh hưởng

- **GDT Setup**: Kernel GDT cần được cập nhật để chứa đúng các selectors Ring 3 code và Ring 3 data liền kề nhau như quy định của lệnh `sysret`.
- **Linker Script Userspace**: Linker script cho các ứng dụng userspace phải chỉ định địa chỉ load tại `0x400000`, build ở dạng ELF `EXEC` static (`-C relocation-model=static`, `-no-pie`) và căn `PT_LOAD` theo page 4 KiB để các segment không ghi đè cùng một trang khi kernel loader map từng segment.
- **Kernel Paging**: Kernel cần hỗ trợ ánh xạ trang ảo với cờ `User` (`U`) để mã Ring 3 có thể truy cập được, đồng thời đảm bảo không gian địa chỉ Kernel (`HHDM` và Kernel Image) được bảo vệ (không có cờ `U`).
- **An toàn bộ nhớ**: Syscall handler phải kiểm tra kỹ lượng các con trỏ từ userspace truyền vào (ví dụ buffer trong `sys_write`), đảm bảo chúng nằm hoàn toàn trong vùng nhớ userspace và không trỏ vào không gian của Kernel để tránh lỗ hổng bảo mật.

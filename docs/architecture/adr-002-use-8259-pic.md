# ADR 002: Sử dụng bộ điều khiển ngắt 8259 PIC trong giai đoạn đầu

* **Status**: ACCEPTED
* **Người đề xuất**: Kỹ sư trưởng AxiomOS
* **Ngày tạo**: 2026-07-07
* **Ngày cập nhật**: 2026-07-07

---

## Bối cảnh (Context)

Để hỗ trợ nhận ngắt từ các thiết bị ngoại vi thiết yếu (như timer ngắt để lập lịch và keyboard ngắt để nhận dữ liệu gõ phím), kernel cần cấu hình bộ điều khiển ngắt (Interrupt Controller). 

Trên kiến trúc x86_64, có hai lựa chọn chính:
1. **Intel 8259 PIC (Programmable Interrupt Controller)**: Hệ thống ngắt truyền thống gồm hai chip PIC (Master và Slave) quản lý 15 dòng ngắt IRQ. Giao tiếp hoàn toàn qua các cổng I/O port thô (`0x20`, `0x21`, `0xA0`, `0xA1`).
2. **APIC (Advanced Programmable Interrupt Controller)**: Hệ thống ngắt hiện đại gồm Local APIC cho mỗi nhân CPU và I/O APIC để định tuyến ngắt từ thiết bị. APIC bắt buộc cho hệ thống đa lõi (SMP), nhưng yêu cầu cơ chế ACPI parser phức tạp để định vị địa chỉ phần cứng và cấu hình các bảng ACPI nâng cao.

## Quyết định (Decision)

Chúng tôi quyết định **sử dụng bộ điều khiển ngắt 8259 PIC truyền thống** cho Milestone 2 để triển khai hệ thống ngắt sớm. 

Các lý do chính bao gồm:
* **Độ phức tạp cực kỳ thấp**: 8259 PIC có thể được cấu hình hoàn toàn chỉ bằng việc ghi một vài byte lệnh khởi tạo (ICW) ra các cổng I/O. Chúng ta không cần phụ thuộc vào ACPI parser hay quản lý vùng nhớ MMIO cho Local APIC/IOAPIC ở giai đoạn sớm này khi virtual memory paging chưa hoàn thiện.
* **Độc lập và nhanh chóng**: Giúp chúng ta nhanh chóng kiểm chứng được IDT, exceptions và ngắt bàn phím (Milestone 2) mà không bị vướng lại quá lâu ở phần hạ tầng ACPI.

**Kế hoạch nâng cấp tương lai:**
* Việc chuyển đổi sang APIC sẽ được dời sang cột mốc phát triển đa lõi (SMP) tiếp theo, khi mà Virtual Memory Paging và Heap Allocator đã ổn định. Khi đó, APIC sẽ được kích hoạt và 8259 PIC sẽ bị vô hiệu hóa (disabled).

## Hệ quả (Consequences)

* **Ưu điểm**:
  * Giảm thiểu hàng trăm dòng code khởi tạo ACPI sớm.
  * Tận dụng được các driver PIC sẵn có và đơn giản.
  * Nhận được ngắt timer và bàn phím nhanh chóng trong QEMU.
* **Nhược điểm**:
  * Bị giới hạn ở chế độ đơn nhân (Single-core).
  * 8259 PIC lỗi thời và có hiệu năng thấp hơn APIC trên máy thật, nhưng hoàn toàn chấp nhận được trong môi trường giả lập QEMU phục vụ nghiên cứu ở Milestone 2.

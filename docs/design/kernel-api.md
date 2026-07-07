# Design: Kernel API

Tài liệu này đặc tả giao diện lập trình ứng dụng (API) và cuộc gọi hệ thống (Syscalls) của Kernel.

*(Skeleton)*

## Trạng thái ABI

AxiomOS hiện chưa công bố kernel ABI ổn định cho userspace. Các interface trong
`kernel/src/memory` như `init_memory`, `allocate_frame`, `deallocate_frame`,
`memory_stats` và `hhdm_offset` là internal kernel API phục vụ Milestone 3.

Mọi thay đổi ABI userspace sau này phải cập nhật tài liệu này, spec liên quan,
ADR liên quan và `CHANGELOG.md`.

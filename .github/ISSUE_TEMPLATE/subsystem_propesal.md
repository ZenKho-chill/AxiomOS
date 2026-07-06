name: Đề xuất subsystem (Subsystem Proposal)
description: Đề xuất một subsystem mới cho AxiomOS theo quy trình Spec Kit
title: '[SUBSYSTEM] '
labels: architecture
assignees: ''
body:
  - type: markdown
    attributes:
      value: |
        Mọi subsystem mới phải có spec APPROVED trước khi implement.
  - type: input
    id: spec-id
    attributes:
      label: Spec ID liên quan
      description: Ví dụ `003-framebuffer-console`.
      placeholder: '003-framebuffer-console'
    validations:
      required: true
  - type: textarea
    id: problem
    attributes:
      label: Vấn đề cần giải quyết
      description: Mô tả vấn đề kỹ thuật mà subsystem này cần giải quyết.
    validations:
      required: true
  - type: textarea
    id: scope
    attributes:
      label: Phạm vi
      description: Liệt kê phạm vi, non-goals, dependency và milestone liên quan.
    validations:
      required: true
  - type: textarea
    id: test-plan
    attributes:
      label: Kế hoạch test
      description: Mô tả cách kiểm chứng trong QEMU hoặc hardware test plan nếu cần.
    validations:
      required: true

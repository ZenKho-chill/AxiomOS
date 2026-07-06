name: Báo cáo lỗi (Bug Report)
description: Báo cáo lỗi xảy ra trong quá trình xây dựng hoặc vận hành AxiomOS
title: '[BUG] '
labels: bug
assignees: ''
body:
  - type: markdown
    attributes:
      value: |
        Cảm ơn bạn đã báo cáo lỗi! Vui lòng cung cấp thông tin bên dưới.
  - type: textarea
    id: description
    attributes:
      label: Mô tả lỗi
      description: Mô tả rõ ràng lỗi là gì.
    validations:
      required: true
  - type: textarea
    id: steps
    attributes:
      label: Các bước tái hiện lỗi
      description: Liệt kê các bước để tái hiện lỗi này.
  - type: textarea
    id: environment
    attributes:
      label: Môi trường
      description: Hệ điều hành host, phiên bản QEMU, phiên bản Rust...

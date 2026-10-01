# ADR-0011: Anker USB-C Ethernet (RTL8153) as the first network device

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

The laptop has no Ethernet port, and its Wi-Fi chip (RTL8852BE-VT, driven by Linux as the 8852BT variant) has no public datasheet and needs firmware blobs. The owner has an Anker USB-C Ethernet adapter, identified as Realtek RTL8153 (`0bda:8153`).

## Decision

Network through the **Anker adapter first**, using a generic **USB CDC-ECM** driver (with CDC-NCM next). The Wi-Fi driver comes later (v0.11).

## Consequences

- Networking arrives much earlier and is testable in QEMU (`usb-net` emulates CDC-ECM).
- Requires the xHCI driver first, which is needed anyway for the USB boot stick.
- No wireless networking until v0.11.

## Alternatives considered

- Wi-Fi driver first: very hard, blob-dependent.
- USB Wi-Fi dongle: extra hardware and still a Wi-Fi stack.

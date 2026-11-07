# macOS Metal Swift Bridge Fix

Source: crates.io `apple-metal` 0.9.0.

In `swift-bridge/Sources/AppleMetalBridge/State.swift`, `MTLSamplerDescriptor.reductionMode` and `lodBias` were guarded by a runtime check `if #available(macOS 26.0, *)` instead of compile-time conditional compilation. Because `MTLSamplerReductionMode` is not exposed in the macOS Metal SDK, the Swift compiler fails to compile the bridge on macOS 14 and macOS 15 SDKs.

The patch comments out the unsupported descriptor properties in `State.swift` to allow `apple-metal` to compile cleanly on macOS.

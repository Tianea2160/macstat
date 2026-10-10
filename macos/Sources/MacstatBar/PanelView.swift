import AppKit
import CMacstat
import SwiftUI

struct PanelView: View {
    let model: StatsModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("macstat")
                .font(.title3.bold())

            if let cpu = model.cpu {
                CpuSection(cpu: cpu)
            }
            if let memory = model.memory {
                MemorySection(memory: memory)
            }
            if model.cpu == nil && model.memory == nil {
                Text("Measuring…")
                    .foregroundStyle(.secondary)
            }

            Divider()

            Button("Quit macstat") {
                NSApplication.shared.terminate(nil)
            }
            .keyboardShortcut("q")
        }
        .padding(16)
        .frame(width: 300)
    }
}

private struct CpuSection: View {
    let cpu: MacstatCpu

    var body: some View {
        Card(title: "CPU", detail: "\(cpu.cores) cores") {
            UsageBar(fraction: (100 - cpu.idle) / 100, tint: .blue)
            StatRow(label: "System", value: percent(cpu.system))
            StatRow(label: "User", value: percent(cpu.user))
            StatRow(label: "Idle", value: percent(cpu.idle))
        }
    }
}

private struct MemorySection: View {
    let memory: MacstatMemory

    var body: some View {
        Card(title: "Memory", detail: "Pressure: \(pressureName)") {
            UsageBar(fraction: Double(memory.used) / Double(memory.total), tint: pressureTint)
            StatRow(label: "Used", value: "\(bytes(memory.used)) of \(bytes(memory.total))")
            StatRow(label: "App", value: bytes(memory.app), indented: true)
            StatRow(label: "Wired", value: bytes(memory.wired), indented: true)
            StatRow(label: "Compressed", value: bytes(memory.compressed), indented: true)
            StatRow(label: "Cached", value: bytes(memory.cached))
            StatRow(label: "Free", value: bytes(memory.free))
            StatRow(label: "Swap", value: "\(bytes(memory.swap_used)) of \(bytes(memory.swap_total))")
        }
    }

    private var pressureName: String {
        switch memory.pressure {
        case 1: "Normal"
        case 2: "Warning"
        case 4: "Critical"
        default: "Unknown (\(memory.pressure))"
        }
    }

    private var pressureTint: Color {
        switch memory.pressure {
        case 2: .orange
        case 4: .red
        default: .green
        }
    }
}

private struct Card<Content: View>: View {
    let title: String
    let detail: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(title).font(.headline)
                Spacer()
                Text(detail).font(.caption).foregroundStyle(.secondary)
            }
            content
        }
        .padding(12)
        .background(.quaternary.opacity(0.5), in: RoundedRectangle(cornerRadius: 10))
    }
}

private struct UsageBar: View {
    let fraction: Double
    let tint: Color

    var body: some View {
        ProgressView(value: min(max(fraction, 0), 1))
            .tint(tint)
    }
}

private struct StatRow: View {
    let label: String
    let value: String
    var indented = false

    var body: some View {
        HStack {
            Text(label)
                .foregroundStyle(.secondary)
                .padding(.leading, indented ? 12 : 0)
            Spacer()
            Text(value)
                .monospacedDigit()
        }
        .font(.callout)
    }
}

private func percent(_ value: Double) -> String {
    String(format: "%.1f%%", value)
}

private func bytes(_ value: UInt64) -> String {
    Int64(clamping: value).formatted(.byteCount(style: .memory))
}

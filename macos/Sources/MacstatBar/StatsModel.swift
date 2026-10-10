import CMacstat
import Foundation
import Observation

@MainActor
@Observable
final class StatsModel {
    private(set) var cpu: MacstatCpu?
    private(set) var memory: MacstatMemory?

    init() {
        Task { await poll() }
    }

    var menuBarTitle: String {
        guard let cpu else { return "macstat" }
        return String(format: "CPU %.0f%%", 100 - cpu.idle)
    }

    private func poll() async {
        while !Task.isCancelled {
            let cpu = await Task.detached { macstat_cpu_measure(1000) }.value
            let memory = macstat_memory_read()
            self.cpu = cpu.ok ? cpu : nil
            self.memory = memory.ok ? memory : nil
        }
    }
}

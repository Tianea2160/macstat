import AppKit
import SwiftUI

@main
struct MacstatBarApp: App {
    @State private var model = StatsModel()

    init() {
        NSApplication.shared.setActivationPolicy(.accessory)
    }

    var body: some Scene {
        MenuBarExtra {
            PanelView(model: model)
        } label: {
            Text(model.menuBarTitle)
        }
        .menuBarExtraStyle(.window)
    }
}

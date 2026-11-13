import Foundation
import GSX
import SwiftUI
import Testing

@Test func modelTypesDoNotClashWithFoundationOrSwiftUI() throws {
    let projection = try GSX.project("a", target: "x-plain")
    let measurement: GSX.Measurement = try #require(projection.measurements.first)
    let unit: GSX.Unit = measurement.unit
    #expect(unit == .xWeighted)

    let media = [GSX.Media(id: "a"), GSX.Media(id: "b")]
    let document: GSX.Document = try GSX.parse("::media{ids=\"a,b\" layout=\"collage\"}", media: media)
    let layout: GSX.Layout = .collage
    #expect(document.children.first?.kind == .mediaGroup(ids: ["a", "b"], layout: layout))

    let length = Measurement(value: 2, unit: UnitLength.meters)
    #expect(length.converted(to: .centimeters).value == 200)
    let stack: any Layout = HStackLayout()
    #expect(stack is HStackLayout)
}

import GSX
import Testing

@Test func exposesMediaBounds() {
    #expect(GSX.mediaIdMaximumBytes == 128)
    #expect(GSX.mediaIdPattern == #"^[A-Za-z0-9][A-Za-z0-9._:\-]{0,127}$"#)
    #expect(GSX.mediaGroupMinimumCount == 2)
    #expect(GSX.mediaGroupMaximumCount == 64)
}

@Test func validatesMediaIds() {
    let longest = String(repeating: "a", count: 128)
    for id in ["p", "0", "Photo-1", "a._:-z", longest] {
        #expect(GSX.isMediaId(id), "\(id)")
    }
    for id in ["", ".a", "-a", "_a", ":a", "a b", "a,b", "a\"b", "a\u{0}b", "é", "aé", "фото", longest + "a"] {
        #expect(!GSX.isMediaId(id), "\(id)")
    }
}

@Test func validationMatchesTheParser() throws {
    for id in ["p", ".p", String(repeating: "a", count: 128), String(repeating: "a", count: 129)] {
        let document = try GSX.parse("::media{id=\"\(id)\"}", media: [GSX.Media(id: id)])
        let parsed = document.children.first?.kind == .mediaReference(id: id)
        #expect(parsed == GSX.isMediaId(id), "\(id)")
    }
}

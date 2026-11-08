import GSX
import Testing

@Test func parsesNodesWithUTF16Ranges() throws {
    let document = try GSX.parse("😀 **a**")
    let paragraph = try #require(document.children.first)
    #expect(paragraph.kind == .paragraph)
    #expect(paragraph.children.map(\.kind) == [.text(value: "😀 "), .strong])
    #expect(paragraph.children[1].range == 3..<8)
    #expect(document.markers.map(\.range) == [3..<5, 6..<8])
}

@Test func passesMedia() throws {
    let media = [Media(id: "v", cover: "c"), Media(id: "c")]
    let document = try GSX.parse("::media{id=\"v\"}", media: media)
    #expect(document.children.first?.kind == .mediaReference(id: "v"))
    let unknown = try GSX.parse("::media{id=\"x\"}", media: media)
    #expect(unknown.diagnostics.first?.code == .mediaReferenceUnknown)
}

@Test func throwsParseErrors() {
    #expect(throws: ParseError.self) { try GSX.parse("a\u{0}b") }
    do {
        _ = try GSX.parse("😀\u{1}")
    } catch let error as ParseError {
        #expect(error.range == 2..<3)
        #expect(error.details.codePoint == 1)
    } catch {
        Issue.record(error)
    }
    do {
        _ = try GSX.parse(String(repeating: ">", count: 65))
    } catch let error as ParseError {
        #expect(error.code == .markdownTooComplex)
        #expect(error.details.limit == "containerDepth")
    } catch {
        Issue.record(error)
    }
}

@Test func projectsWithFacts() throws {
    let plain = try GSX.project("# Hi\n\n*a*", target: "x-plain")
    #expect(plain.fields.map(\.text) == ["Hi\n\na"])
    #expect(plain.changes.first?.kind == .textualFallback)
    #expect(plain.measurements.first?.unit == .xWeighted)

    let facts = Facts(media: .video, mediaCount: 1, title: "T")
    let video = try GSX.project("a\n\n::media{id=\"p1\"}", target: "youtube-plain", facts: facts, media: [Media(id: "p1")])
    #expect(video.fields.first { $0.name == .title }?.text == "T")
    #expect(video.media == ["p1"])

    let link = try GSX.project("**a** [b](https://c.example)", target: "bluesky-entities")
    let entity = try #require(link.fields.first?.entities.first)
    #expect(entity.style == .link && entity.range == 2..<3 && entity.value == "https://c.example/")
    #expect(try GSX.project("", target: "x-plain").changes.first?.code == "TEXT_EMPTY")

    #expect(throws: InternalError.self) { try GSX.projectJSON("a", target: "nope") }
    #expect(throws: ParseError.self) { try GSX.project("a\u{0}b", target: "x-plain") }
}

@Test func laysOutBlocksWithMediaFacts() throws {
    let gif = MediaFact(id: "p", format: .gif, alt: "A cat", spoiler: true, inline: true)
    let facts = Facts(media: .image, mediaCount: 1, items: [gif])
    let blocks = try GSX.project("a\n\n::media{id=\"p\"}", target: "discord-blocks", facts: facts, media: [Media(id: "p")])
    let laid = try #require(blocks.blocks)
    #expect(laid.first == .text("a\n\n"))
    guard case .media(let items, let layout) = laid.last else { Issue.record("no media block in \(laid)"); return }
    #expect(items.map(\.reference) == ["media:p"] && items.first?.spoiler == true && layout == nil)
    let audio = Facts(media: .audio, mediaCount: 1, items: [MediaFact(id: "s", format: .audio, inline: true)])
    let filed = try GSX.project("::media{id=\"s\"}", target: "discord-blocks", facts: audio, media: [Media(id: "s")])
    guard case .file(let file) = filed.blocks?.first else { Issue.record("no file block in \(String(describing: filed.blocks))"); return }
    #expect(file.reference == "media:s" && file.alt == nil)
    #expect(try GSX.project("a", target: "x-plain").blocks == nil)
    #expect(GSX.targets.contains { $0.name == "discord-blocks" && $0.form == .blocks })
}

@Test func listsTargets() {
    #expect(GSX.targets.contains { $0.name == "bluesky-entities" && $0.form == .entities && $0.offsets == .utf8 })
    #expect(GSX.targets.allSatisfy { $0.digest.count == 16 && !$0.manifest.isEmpty })
}

@Test func returnsTheWireJSON() throws {
    #expect(try GSX.parseJSON("a") == #"{"document":{"children":[{"type":"paragraph","range":[0,1],"gapBefore":0,"children":[{"type":"text","range":[0,1],"value":"a"}]}],"markers":[],"diagnostics":[]}}"#)
    #expect(!GSX.version.isEmpty)
}

import XCTest
import SwiftTreeSitter
import TreeSitterQed

final class TreeSitterQedTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_qed())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Qed grammar")
    }
}

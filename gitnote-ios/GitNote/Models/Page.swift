import Foundation

struct Page: Codable, Identifiable, Equatable {
    let id: String
    var title: String
    var content: String
    var tags: [String]
    var parentId: String?
    var icon: String?
    let createdAt: Date
    let updatedAt: Date

    enum CodingKeys: String, CodingKey {
        case id, title, content, tags, icon
        case parentId = "parent_id"
        case createdAt = "created_at"
        case updatedAt = "updated_at"
    }
}

struct PageSummary: Codable, Identifiable, Equatable {
    let id: String
    var title: String
    var tags: [String]
    var parentId: String?
    var icon: String?
    let createdAt: Date
    let updatedAt: Date

    enum CodingKeys: String, CodingKey {
        case id, title, tags, icon
        case parentId = "parent_id"
        case createdAt = "created_at"
        case updatedAt = "updated_at"
    }
}

struct PageListResponse: Codable {
    let pages: [PageSummary]
    let total: Int
}

struct CommitInfo: Codable, Identifiable {
    let oid: String
    let message: String
    let timestamp: Date

    var id: String { oid }
}

struct SearchResult: Codable {
    let pages: [PageSummary]
    let total: Int
}

struct CreatePageRequest: Codable {
    let title: String
    let content: String
    let tags: [String]
    let icon: String?
}

struct UpdatePageRequest: Codable {
    let title: String
    let content: String
    let tags: [String]
    let icon: String?
}

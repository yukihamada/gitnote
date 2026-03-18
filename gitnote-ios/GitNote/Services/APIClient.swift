import Foundation

@Observable
final class APIClient {
    var baseURL: String
    var isLoading = false
    var errorMessage: String?

    private let session: URLSession
    private let decoder: JSONDecoder
    private let encoder: JSONEncoder

    init(baseURL: String = "http://localhost:3000") {
        self.baseURL = baseURL

        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 30
        self.session = URLSession(configuration: config)

        self.decoder = JSONDecoder()
        self.decoder.dateDecodingStrategy = .iso8601

        self.encoder = JSONEncoder()
        self.encoder.dateEncodingStrategy = .iso8601
    }

    // MARK: - Pages

    func listPages() async throws -> [PageSummary] {
        let data = try await get(path: "/api/pages")
        let response = try decoder.decode(PageListResponse.self, from: data)
        return response.pages
    }

    func getPage(id: String) async throws -> Page {
        let data = try await get(path: "/api/pages/\(id)")
        return try decoder.decode(Page.self, from: data)
    }

    func createPage(title: String, content: String = "", tags: [String] = [], icon: String? = nil) async throws -> Page {
        let body = CreatePageRequest(title: title, content: content, tags: tags, icon: icon)
        let data = try await post(path: "/api/pages", body: body)
        return try decoder.decode(Page.self, from: data)
    }

    func updatePage(id: String, title: String, content: String, tags: [String] = [], icon: String? = nil) async throws -> Page {
        let body = UpdatePageRequest(title: title, content: content, tags: tags, icon: icon)
        let data = try await put(path: "/api/pages/\(id)", body: body)
        return try decoder.decode(Page.self, from: data)
    }

    func deletePage(id: String) async throws {
        _ = try await delete(path: "/api/pages/\(id)")
    }

    // MARK: - History

    func getHistory(pageId: String) async throws -> [CommitInfo] {
        let data = try await get(path: "/api/pages/\(pageId)/history")
        return try decoder.decode([CommitInfo].self, from: data)
    }

    func getRevision(pageId: String, oid: String) async throws -> Page {
        let data = try await get(path: "/api/pages/\(pageId)/revisions/\(oid)")
        return try decoder.decode(Page.self, from: data)
    }

    func restoreRevision(pageId: String, oid: String) async throws -> Page {
        let data = try await post(path: "/api/pages/\(pageId)/restore/\(oid)", body: Optional<String>.none)
        return try decoder.decode(Page.self, from: data)
    }

    // MARK: - Search

    func search(query: String) async throws -> [PageSummary] {
        guard let encoded = query.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) else {
            throw APIError.invalidURL
        }
        let data = try await get(path: "/api/search?q=\(encoded)")
        let response = try decoder.decode(SearchResult.self, from: data)
        return response.pages
    }

    // MARK: - HTTP Methods

    private func get(path: String) async throws -> Data {
        let request = try buildRequest(path: path, method: "GET")
        return try await execute(request)
    }

    private func post<T: Encodable>(path: String, body: T?) async throws -> Data {
        var request = try buildRequest(path: path, method: "POST")
        if let body {
            request.httpBody = try encoder.encode(body)
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }
        return try await execute(request)
    }

    private func put<T: Encodable>(path: String, body: T) async throws -> Data {
        var request = try buildRequest(path: path, method: "PUT")
        request.httpBody = try encoder.encode(body)
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        return try await execute(request)
    }

    private func delete(path: String) async throws -> Data {
        let request = try buildRequest(path: path, method: "DELETE")
        return try await execute(request)
    }

    private func buildRequest(path: String, method: String) throws -> URLRequest {
        guard let url = URL(string: baseURL + path) else {
            throw APIError.invalidURL
        }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        return request
    }

    private func execute(_ request: URLRequest) async throws -> Data {
        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw APIError.invalidResponse
        }
        guard (200...299).contains(http.statusCode) else {
            let body = String(data: data, encoding: .utf8) ?? ""
            throw APIError.httpError(statusCode: http.statusCode, body: body)
        }
        return data
    }
}

enum APIError: LocalizedError {
    case invalidURL
    case invalidResponse
    case httpError(statusCode: Int, body: String)

    var errorDescription: String? {
        switch self {
        case .invalidURL:
            return "Invalid URL"
        case .invalidResponse:
            return "Invalid server response"
        case .httpError(let code, let body):
            return "HTTP \(code): \(body)"
        }
    }
}

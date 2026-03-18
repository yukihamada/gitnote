import SwiftUI

struct SearchView: View {
    @Environment(APIClient.self) private var apiClient
    @Binding var selectedPageId: String?
    @Binding var showSearch: Bool

    @State private var query: String = ""
    @State private var results: [PageSummary] = []
    @State private var isSearching = false
    @State private var hasSearched = false
    @State private var searchTask: Task<Void, Never>?

    var body: some View {
        NavigationStack {
            List(results) { page in
                Button {
                    selectedPageId = page.id
                    showSearch = false
                } label: {
                    HStack(spacing: 12) {
                        Text(page.icon ?? "📄")
                            .font(.title3)

                        VStack(alignment: .leading, spacing: 4) {
                            Text(page.title.isEmpty ? "Untitled" : page.title)
                                .font(.body)
                                .fontWeight(.medium)
                                .foregroundStyle(.primary)

                            if !page.tags.isEmpty {
                                Text(page.tags.joined(separator: " ・ "))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            Text(page.updatedAt, style: .relative)
                                .font(.caption2)
                                .foregroundStyle(.tertiary)
                        }
                    }
                    .padding(.vertical, 2)
                }
            }
            .overlay {
                if isSearching {
                    ProgressView()
                } else if hasSearched && results.isEmpty && !query.isEmpty {
                    ContentUnavailableView.search(text: query)
                } else if query.isEmpty {
                    ContentUnavailableView(
                        "Search Pages",
                        systemImage: "magnifyingglass",
                        description: Text("Type to search across all your pages.")
                    )
                }
            }
            .navigationTitle("Search")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button("Cancel") {
                        showSearch = false
                    }
                }
            }
        }
        .searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always), prompt: "Search pages...")
        .onChange(of: query) { _, newValue in
            scheduleSearch(query: newValue)
        }
    }

    private func scheduleSearch(query: String) {
        searchTask?.cancel()
        guard !query.trimmingCharacters(in: .whitespaces).isEmpty else {
            results = []
            hasSearched = false
            return
        }
        searchTask = Task {
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled else { return }
            await performSearch(query: query)
        }
    }

    private func performSearch(query: String) async {
        isSearching = true
        defer {
            isSearching = false
            hasSearched = true
        }
        do {
            results = try await apiClient.search(query: query)
        } catch {
            if !Task.isCancelled {
                results = []
            }
        }
    }
}

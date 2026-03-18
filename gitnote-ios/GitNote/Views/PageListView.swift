import SwiftUI

struct PageListView: View {
    @Environment(APIClient.self) private var apiClient
    @Binding var pages: [PageSummary]
    @Binding var selectedPageId: String?
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        List(selection: $selectedPageId) {
            ForEach(pages) { page in
                NavigationLink(value: page.id) {
                    PageRow(page: page)
                }
            }
            .onDelete(perform: deletePages)
        }
        .listStyle(.insetGrouped)
        .navigationTitle("GitNote")
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    Task { await createNewPage() }
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .refreshable {
            await loadPages()
        }
        .overlay {
            if isLoading && pages.isEmpty {
                ProgressView()
            } else if pages.isEmpty && !isLoading {
                ContentUnavailableView(
                    "No Pages",
                    systemImage: "note.text",
                    description: Text("Tap + to create your first page.")
                )
            }
        }
        .alert("Error", isPresented: .init(
            get: { errorMessage != nil },
            set: { if !$0 { errorMessage = nil } }
        )) {
            Button("OK") { errorMessage = nil }
        } message: {
            Text(errorMessage ?? "")
        }
        .task {
            await loadPages()
        }
    }

    private func loadPages() async {
        isLoading = true
        defer { isLoading = false }
        do {
            pages = try await apiClient.listPages()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func createNewPage() async {
        do {
            let page = try await apiClient.createPage(title: "Untitled")
            await loadPages()
            selectedPageId = page.id
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func deletePages(at offsets: IndexSet) {
        let idsToDelete = offsets.map { pages[$0].id }
        Task {
            for id in idsToDelete {
                do {
                    try await apiClient.deletePage(id: id)
                } catch {
                    errorMessage = error.localizedDescription
                    return
                }
            }
            await loadPages()
            if let selected = selectedPageId, idsToDelete.contains(selected) {
                selectedPageId = nil
            }
        }
    }
}

private struct PageRow: View {
    let page: PageSummary

    var body: some View {
        HStack(spacing: 12) {
            Text(page.icon ?? "📄")
                .font(.title2)

            VStack(alignment: .leading, spacing: 4) {
                Text(page.title.isEmpty ? "Untitled" : page.title)
                    .font(.body)
                    .fontWeight(.medium)
                    .lineLimit(1)

                if !page.tags.isEmpty {
                    Text(page.tags.joined(separator: " ・ "))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }

                Text(page.updatedAt, style: .relative)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 4)
    }
}

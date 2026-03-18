import SwiftUI

struct PageEditorView: View {
    @Environment(APIClient.self) private var apiClient
    let pageId: String

    @State private var page: Page?
    @State private var title: String = ""
    @State private var content: String = ""
    @State private var tags: [String] = []
    @State private var icon: String?
    @State private var isLoading = true
    @State private var isSaving = false
    @State private var errorMessage: String?
    @State private var showHistory = false
    @State private var saveTask: Task<Void, Never>?
    @State private var lastSavedTitle: String = ""
    @State private var lastSavedContent: String = ""

    var body: some View {
        Group {
            if isLoading {
                ProgressView()
            } else if let _ = page {
                editorContent
            } else {
                ContentUnavailableView(
                    "Failed to Load",
                    systemImage: "exclamationmark.triangle",
                    description: Text(errorMessage ?? "Unknown error")
                )
            }
        }
        .navigationTitle(title.isEmpty ? "Untitled" : title)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                HStack(spacing: 16) {
                    if isSaving {
                        ProgressView()
                            .controlSize(.small)
                    }

                    Button {
                        showHistory = true
                    } label: {
                        Image(systemName: "clock.arrow.circlepath")
                    }
                }
            }
        }
        .sheet(isPresented: $showHistory) {
            NavigationStack {
                HistoryView(pageId: pageId) { restoredPage in
                    applyPage(restoredPage)
                }
            }
        }
        .task {
            await loadPage()
        }
    }

    private var editorContent: some View {
        VStack(spacing: 0) {
            // Title field
            TextField("Title", text: $title)
                .font(.title)
                .fontWeight(.bold)
                .padding(.horizontal, 16)
                .padding(.top, 16)
                .padding(.bottom, 8)
                .onChange(of: title) { _, _ in
                    scheduleSave()
                }

            // Tags display
            if !tags.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(tags, id: \.self) { tag in
                            Text(tag)
                                .font(.caption)
                                .padding(.horizontal, 10)
                                .padding(.vertical, 4)
                                .background(.fill.tertiary)
                                .clipShape(Capsule())
                        }
                    }
                    .padding(.horizontal, 16)
                }
                .padding(.bottom, 8)
            }

            Divider()
                .padding(.horizontal, 16)

            // Content editor
            TextEditor(text: $content)
                .font(.body.monospaced())
                .scrollContentBackground(.hidden)
                .padding(.horizontal, 12)
                .padding(.top, 8)
                .onChange(of: content) { _, _ in
                    scheduleSave()
                }
        }
    }

    private func loadPage() async {
        isLoading = true
        defer { isLoading = false }
        do {
            let loaded = try await apiClient.getPage(id: pageId)
            applyPage(loaded)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func applyPage(_ p: Page) {
        page = p
        title = p.title
        content = p.content
        tags = p.tags
        icon = p.icon
        lastSavedTitle = p.title
        lastSavedContent = p.content
    }

    private func scheduleSave() {
        saveTask?.cancel()
        saveTask = Task {
            try? await Task.sleep(for: .seconds(2))
            guard !Task.isCancelled else { return }
            guard title != lastSavedTitle || content != lastSavedContent else { return }
            await save()
        }
    }

    private func save() async {
        isSaving = true
        defer { isSaving = false }
        do {
            let updated = try await apiClient.updatePage(
                id: pageId,
                title: title,
                content: content,
                tags: tags,
                icon: icon
            )
            page = updated
            lastSavedTitle = title
            lastSavedContent = content
        } catch {
            if !Task.isCancelled {
                errorMessage = error.localizedDescription
            }
        }
    }
}

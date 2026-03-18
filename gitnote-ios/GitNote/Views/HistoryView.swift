import SwiftUI

struct HistoryView: View {
    @Environment(APIClient.self) private var apiClient
    @Environment(\.dismiss) private var dismiss
    let pageId: String
    var onRestore: (Page) -> Void

    @State private var commits: [CommitInfo] = []
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var selectedCommit: CommitInfo?
    @State private var revisionPage: Page?
    @State private var isLoadingRevision = false
    @State private var isRestoring = false

    var body: some View {
        Group {
            if isLoading {
                ProgressView()
            } else if commits.isEmpty {
                ContentUnavailableView(
                    "No History",
                    systemImage: "clock",
                    description: Text("This page has no commit history yet.")
                )
            } else {
                historyList
            }
        }
        .navigationTitle("History")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button("Done") { dismiss() }
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
        .sheet(item: $selectedCommit) { commit in
            NavigationStack {
                RevisionDetailView(
                    commit: commit,
                    page: revisionPage,
                    isLoading: isLoadingRevision,
                    isRestoring: isRestoring
                ) {
                    await restore(oid: commit.oid)
                }
            }
        }
        .task {
            await loadHistory()
        }
    }

    private var historyList: some View {
        List(commits) { commit in
            Button {
                Task {
                    selectedCommit = commit
                    await loadRevision(oid: commit.oid)
                }
            } label: {
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(commit.message)
                            .font(.body)
                            .foregroundStyle(.primary)
                            .lineLimit(2)

                        Text(commit.timestamp, style: .relative)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                    Spacer()

                    Text(String(commit.oid.prefix(7)))
                        .font(.caption.monospaced())
                        .foregroundStyle(.tertiary)
                }
                .padding(.vertical, 4)
            }
        }
    }

    private func loadHistory() async {
        isLoading = true
        defer { isLoading = false }
        do {
            commits = try await apiClient.getHistory(pageId: pageId)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func loadRevision(oid: String) async {
        isLoadingRevision = true
        revisionPage = nil
        defer { isLoadingRevision = false }
        do {
            revisionPage = try await apiClient.getRevision(pageId: pageId, oid: oid)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func restore(oid: String) async {
        isRestoring = true
        defer { isRestoring = false }
        do {
            let restored = try await apiClient.restoreRevision(pageId: pageId, oid: oid)
            onRestore(restored)
            selectedCommit = nil
            dismiss()
        } catch {
            errorMessage = error.localizedDescription
        }
    }
}

private struct RevisionDetailView: View {
    let commit: CommitInfo
    let page: Page?
    let isLoading: Bool
    let isRestoring: Bool
    let onRestore: () async -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        Group {
            if isLoading {
                ProgressView()
            } else if let page {
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        Text(page.title)
                            .font(.title2)
                            .fontWeight(.bold)

                        Text(commit.message)
                            .font(.subheadline)
                            .foregroundStyle(.secondary)

                        Divider()

                        Text(page.content)
                            .font(.body.monospaced())
                    }
                    .padding()
                }
            } else {
                ContentUnavailableView(
                    "Failed to Load",
                    systemImage: "exclamationmark.triangle"
                )
            }
        }
        .navigationTitle("Revision \(String(commit.oid.prefix(7)))")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button("Close") { dismiss() }
            }
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    Task { await onRestore() }
                } label: {
                    if isRestoring {
                        ProgressView()
                            .controlSize(.small)
                    } else {
                        Label("Restore", systemImage: "arrow.counterclockwise")
                    }
                }
                .disabled(page == nil || isRestoring)
            }
        }
    }
}

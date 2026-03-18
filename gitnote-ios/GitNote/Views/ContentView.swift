import SwiftUI

struct ContentView: View {
    @Environment(APIClient.self) private var apiClient
    @State private var selectedPageId: String?
    @State private var pages: [PageSummary] = []
    @State private var showSearch = false

    var body: some View {
        NavigationSplitView {
            PageListView(
                pages: $pages,
                selectedPageId: $selectedPageId
            )
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button {
                        showSearch = true
                    } label: {
                        Image(systemName: "magnifyingglass")
                    }
                }
            }
            .sheet(isPresented: $showSearch) {
                SearchView(selectedPageId: $selectedPageId, showSearch: $showSearch)
            }
        } detail: {
            if let pageId = selectedPageId {
                PageEditorView(pageId: pageId)
                    .id(pageId)
            } else {
                ContentUnavailableView(
                    "No Page Selected",
                    systemImage: "doc.text",
                    description: Text("Select a page from the sidebar or create a new one.")
                )
            }
        }
    }
}

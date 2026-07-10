import { createApp } from "vue";
import "../assets/theme.css";
import DocumentEditorPage from "../components/results/DocumentEditorPage.vue";
import { installInputUndo } from "../utils/inputUndo";

// This editor is a separate webview, so it loads its own stylesheet and pre-paints
// the theme from the shared localStorage mirror the main window keeps in sync.
document.documentElement.dataset.theme = localStorage.getItem("s4t-theme") || "dark";

createApp(DocumentEditorPage).mount("#doc-editor-app");

// This window is a separate webview, so it needs its own undo shim (WebKitGTK has
// no native Ctrl+Z for text fields).
installInputUndo();

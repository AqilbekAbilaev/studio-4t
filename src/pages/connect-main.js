import { createApp } from "vue";
import "../assets/theme.css";
import ConnectDialog from "../components/connect/ConnectDialog.vue";
import { installInputUndo } from "../utils/inputUndo";

// This popup is a separate webview, so it loads its own stylesheet and pre-paints
// the theme from the shared localStorage mirror the main window keeps in sync.
document.documentElement.dataset.theme = localStorage.getItem("s4t-theme") || "dark";

createApp(ConnectDialog).mount("#connect-app");

// This window is a separate webview, so it needs its own undo shim (WebKitGTK has
// no native Ctrl+Z for text fields).
installInputUndo();

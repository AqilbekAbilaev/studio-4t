import { createApp } from "vue";
import ConnectDialog from "../components/connect/ConnectDialog.vue";
import { installInputUndo } from "../utils/inputUndo";

createApp(ConnectDialog).mount("#connect-app");

// This window is a separate webview, so it needs its own undo shim (WebKitGTK has
// no native Ctrl+Z for text fields).
installInputUndo();

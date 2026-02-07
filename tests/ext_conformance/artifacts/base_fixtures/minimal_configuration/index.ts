import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

export default function(pi: ExtensionAPI) {
    pi.registerFlag("verbose", {
        description: "Enable verbose output",
        type: "boolean",
        default: false
    });

    pi.registerShortcut({ key: "v", modifiers: ["ctrl", "shift"] }, {
        description: "Toggle verbose mode",
        handler: async () => {
            // Toggle verbose flag
        }
    });
}

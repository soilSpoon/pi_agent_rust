import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

export default function(pi: ExtensionAPI) {
    pi.registerMessageRenderer("test/plain", (content: any) => {
        return `[rendered] ${content.text || ""}`;
    });
}
